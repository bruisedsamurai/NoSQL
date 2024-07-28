use crate::util::generate_random_lvl;
use std::borrow::Borrow;
use std::sync::atomic::Ordering;
use std::{convert::TryInto, sync::atomic::AtomicPtr};
use std::{ptr, result};

type KeyType = u64;

#[derive(Debug)]
struct Node<ValueType>
where
    ValueType: Clone,
{
    pub key: KeyType,
    pub value: Option<ValueType>,
    top_level: usize,
    pub next: [AtomicPtr<NodeMarker<ValueType>>; 31],
}

#[derive(Debug)]
struct NodeMarker<ValueType>
where
    ValueType: Clone,
{
    pub marked: bool,
    pub node: *mut Node<ValueType>,
}

impl<ValueType> NodeMarker<ValueType>
where
    ValueType: Clone,
{
    fn empty_new() -> NodeMarker<ValueType> {
        NodeMarker {
            marked: false,
            node: ptr::null_mut(),
        }
    }

    fn boxed_new(marked: bool, node: *mut Node<ValueType>) -> *mut NodeMarker<ValueType> {
        Box::into_raw(Box::new(NodeMarker { marked, node }))
    }
}

impl<ValueType> Node<ValueType>
where
    ValueType: Clone,
{
    fn new_sentinel(key: KeyType) -> Node<ValueType> {
        const TOP_LEVEL: usize = 31;
        let vec = (0..TOP_LEVEL + 1)
            .map(|i| AtomicPtr::new(Box::into_raw(Box::new(NodeMarker::empty_new()))))
            .collect::<Vec<_>>();
        Node {
            key,
            value: None,
            top_level: TOP_LEVEL,
            next: vec.try_into().expect("Cannot convert to array"),
        }
    }

    fn new(key: KeyType, value: ValueType, height: usize) -> Node<ValueType> {
        let vec = (0..height + 1)
            .map(|i| AtomicPtr::new(Box::into_raw(Box::new(NodeMarker::empty_new()))))
            .collect::<Vec<_>>();
        Node {
            key,
            value: Some(value),
            top_level: height,
            next: vec.try_into().expect("Cannot convert to array"),
        }
    }
}

struct SkipList<ValueType>
where
    ValueType: Clone,
{
    head: Node<ValueType>,
    tail: Node<ValueType>,
}

impl<ValueType> SkipList<ValueType>
where
    ValueType: Clone,
{
    const MAX_LEVEL: u64 = 31;

    pub fn new() -> SkipList<ValueType> {
        let mut head: Node<ValueType> = Node::new_sentinel(0);
        let tail: Node<ValueType> = Node::new_sentinel(u64::MAX);
        for i in 0..head.next.len() {
            head.next[i] = AtomicPtr::new(Box::into_raw(Box::new(NodeMarker::empty_new())));
        }
        SkipList { head, tail }
    }

    pub fn add(&mut self, key: KeyType, value: ValueType) -> bool
    where
        ValueType: Clone,
    {
        let top_level = generate_random_lvl(Self::MAX_LEVEL as u64);
        let bottom_level = 0;
        loop {
            let result = self.find(key);
            if result.success {
                return false;
            } else {
                let mut new_node =
                    Box::into_raw(Box::new(Node::new(key, value.clone(), top_level as usize)));
                for level in 0..top_level + 1 {
                    let succ = result.succs[level as usize];
                    unsafe {
                        new_node.as_ref().unwrap().next[level as usize].store(
                            Box::into_raw(Box::new(NodeMarker {
                                node: succ,
                                marked: false,
                            })),
                            Ordering::SeqCst,
                        );
                    }
                }
                let pred = result.preds[bottom_level as usize];
                let succ = result.succs[bottom_level as usize];
                unsafe {
                    if pred.as_ref().unwrap().next[bottom_level as usize]
                        .compare_exchange(
                            NodeMarker::boxed_new(false, succ),
                            NodeMarker::boxed_new(false, new_node),
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        )
                        .is_err()
                    {
                        continue;
                    }
                }
                let mut result = result;
                for level in bottom_level + 1..=top_level {
                    loop {
                        let pred = result.preds[level as usize];
                        let succ = result.succs[level as usize];
                        unsafe {
                            if pred.as_ref().unwrap().next[bottom_level as usize]
                                .compare_exchange(
                                    NodeMarker::boxed_new(false, succ),
                                    NodeMarker::boxed_new(false, new_node),
                                    Ordering::SeqCst,
                                    Ordering::SeqCst,
                                )
                                .is_ok()
                            {
                                break;
                            }
                        }
                        result = self.find(key);
                    }
                }
                return true;
            }
        }
    }

    pub fn remove(&mut self, key: KeyType) -> bool {
        const BOTTOM_LEVEL: usize = 0;
        let mut succ;
        loop {
            let result = self.find(key);
            if !result.success {
                return false;
            } else {
                let node_to_remove = result.succs[BOTTOM_LEVEL];
                let height;
                unsafe {
                    height = node_to_remove.as_ref().unwrap().top_level;
                }
                for level in (BOTTOM_LEVEL + 1..=height).rev() {
                    let mut marked;
                    unsafe {
                        let marker =
                            node_to_remove.as_ref().unwrap().next[level].load(Ordering::SeqCst);
                        succ = marker.as_ref().unwrap().node;
                        marked = marker.as_ref().unwrap().marked;
                        while !marked {
                            let _ = node_to_remove.as_ref().unwrap().next[level].compare_exchange(
                                NodeMarker::boxed_new(false, succ),
                                NodeMarker::boxed_new(true, succ),
                                Ordering::SeqCst,
                                Ordering::SeqCst,
                            );
                            let marker =
                                node_to_remove.as_ref().unwrap().next[level].load(Ordering::SeqCst);
                            succ = marker.as_ref().unwrap().node;
                            marked = marker.as_ref().unwrap().marked;
                        }
                    }
                }
                let mut marked;
                unsafe {
                    let marker =
                        node_to_remove.as_ref().unwrap().next[BOTTOM_LEVEL].load(Ordering::SeqCst);
                    succ = marker.as_ref().unwrap().node;
                    marked = marker.as_ref().unwrap().marked;
                }
                loop {
                    let exchange_result;
                    unsafe {
                        exchange_result = node_to_remove.as_ref().unwrap().next[BOTTOM_LEVEL]
                            .compare_exchange(
                                NodeMarker::boxed_new(false, succ),
                                NodeMarker::boxed_new(true, succ),
                                Ordering::SeqCst,
                                Ordering::SeqCst,
                            );
                        succ = result.succs[BOTTOM_LEVEL].as_ref().unwrap().next[BOTTOM_LEVEL]
                            .load(Ordering::SeqCst)
                            .as_ref()
                            .unwrap()
                            .node;
                    }
                    if exchange_result.is_ok() {
                        let _ = self.find(key);
                        return true;
                    } else if marked {
                        return false;
                    }
                }
            }
        }
    }

    pub fn find(&mut self, key: KeyType) -> FindResult<ValueType> {
        let bottom_level = 0;
        let top_level = Self::MAX_LEVEL;
        let mut snip;
        let mut pred: *mut Node<ValueType>;
        let mut marked: bool;
        let mut curr = ptr::null_mut();
        let mut succ;
        let mut preds = vec![];
        let mut succs = vec![];
        let mut to_be_freed = vec![];
        'retry: loop {
            pred = &mut self.head;
            for lvl in (bottom_level..=Self::MAX_LEVEL).rev() {
                unsafe {
                    let marker = pred.as_ref().unwrap().next[lvl as usize].load(Ordering::SeqCst);
                    curr = marker.as_ref().unwrap().node;
                }
                loop {
                    unsafe {
                        let marker =
                            curr.as_ref().unwrap().next[lvl as usize].load(Ordering::SeqCst);
                        succ = marker.as_ref().unwrap().node;
                        marked = marker.as_ref().unwrap().marked;
                    }
                    while marked {
                        unsafe {
                            to_be_freed.push(curr);
                            snip = pred.as_ref().unwrap().next[lvl as usize].compare_exchange(
                                NodeMarker::boxed_new(false, curr),
                                NodeMarker::boxed_new(false, succ),
                                Ordering::SeqCst,
                                Ordering::SeqCst,
                            )
                        };
                        if snip.is_err() {
                            continue 'retry;
                        }
                        unsafe {
                            let curr_pair =
                                pred.as_ref().unwrap().next[lvl as usize].load(Ordering::SeqCst);
                            curr = curr_pair.as_ref().unwrap().node;
                            let succ =
                                curr.as_ref().unwrap().next[lvl as usize].load(Ordering::SeqCst);
                            marked = succ.as_ref().unwrap().marked;
                        }
                    }
                    unsafe {
                        if curr.as_ref().unwrap().key < key {
                            pred = curr;
                            curr = succ;
                        } else {
                            break;
                        }
                    }
                }
                preds.push(pred);
                succs.push(succ);
            }
            unsafe {
                return FindResult {
                    success: curr.as_ref().unwrap().key == key,
                    preds,
                    succs,
                };
            }
        }
    }
}

struct FindResult<ValueType>
where
    ValueType: Clone,
{
    pub success: bool,
    pub preds: Vec<*mut Node<ValueType>>,
    pub succs: Vec<*mut Node<ValueType>>,
}
