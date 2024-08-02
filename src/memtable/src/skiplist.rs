#[cfg(test)]
mod tests {
    use super::*;
    use rand::distributions::{Alphanumeric, DistString};
    use rand::Rng;

    #[test]
    fn test_skiplist() {
        let mut skiplist = SkipList::new();
        skiplist.add(0, "a");
        skiplist.add(1, "b");
        skiplist.add(2, "c");
        skiplist.add(3, "d");
        skiplist.add(4, "e");
        skiplist.add(5, "f");
        skiplist.add(6, "g");
        skiplist.add(7, "h");
        skiplist.add(8, "i");
        skiplist.add(9, "j");

        let result = skiplist.find(0);
        assert_eq!(result.success, true);
        let result = skiplist.find(1);
        assert!(result.success);
        let result = skiplist.find(2);
        assert!(result.success);
        let result = skiplist.find(3);
        assert!(result.success);
        let result = skiplist.find(4);
        assert!(result.success);
        let result = skiplist.find(5);
        assert!(result.success);
        let result = skiplist.find(6);
        assert!(result.success);
        let result = skiplist.find(7);
        assert!(result.success);
        let result = skiplist.find(8);
        assert!(result.success);
        let result = skiplist.find(9);
        assert!(result.success);
    }

    #[test]
    fn test_skiplist_remove() {
        let mut skiplist = SkipList::new();
        skiplist.add(0, "a");
        skiplist.add(1, "b");
        skiplist.add(2, "c");
        skiplist.add(3, "d");
        skiplist.add(4, "e");
        skiplist.add(5, "f");
        skiplist.add(6, "g");
        skiplist.add(7, "h");
        skiplist.add(8, "i");
        skiplist.add(9, "j");

        let success = skiplist.remove(0);
        assert!(success);
        let result = skiplist.find(0);
        assert!(!result.success);
        let success = skiplist.remove(1);
        assert!(success);
        let result = skiplist.find(1);
        assert!(!result.success);
        let success = skiplist.remove(2);
        assert!(success);
        let result = skiplist.find(2);
        assert!(!result.success);
        let success = skiplist.remove(3);
        assert!(success);
        let result = skiplist.find(3);
        assert!(!result.success);
    }
}
use crate::util::generate_random_lvl;
use std::borrow::Borrow;
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::{
    convert::TryInto,
    sync::atomic::{AtomicPtr, AtomicUsize},
};
use std::{ptr, result};

type KeyType = u64;

#[repr(align(2))]
#[derive(Debug)]
struct Node<ValueType>
where
    ValueType: Clone,
{
    pub key: KeyType,
    pub value: Option<ValueType>,
    top_level: usize,
    pub next: [AtomicPtr<Node<ValueType>>; 32],
}

#[inline(always)]
fn get_node<ValueType>(ptr: *mut Node<ValueType>) -> *mut Node<ValueType>
where
    ValueType: Clone,
{
    (ptr as usize & 0xFFFFFFFFFFFFFFFE) as *mut Node<ValueType>
}

fn get_marker<ValueType>(ptr: *mut Node<ValueType>) -> bool
where
    ValueType: Clone,
{
    (ptr as usize & 0x1) == 0x1
}

fn add_marker<ValueType>(ptr: *mut Node<ValueType>, marker: bool) -> *mut Node<ValueType>
where
    ValueType: Clone,
{
    if marker {
        (ptr as usize | 0x1) as *mut Node<ValueType>
    } else {
        ptr
    }
}

impl<ValueType> Node<ValueType>
where
    ValueType: Clone,
{
    fn new_sentinel(key: KeyType) -> *mut Node<ValueType> {
        const TOP_LEVEL: usize = 31;
        let vec = (0..TOP_LEVEL + 1)
            .map(|i| AtomicPtr::new(std::ptr::null_mut()))
            .collect::<Vec<_>>();
        Box::into_raw(Box::new(Node {
            key,
            value: None,
            top_level: TOP_LEVEL,
            next: vec.try_into().expect("Cannot convert to array"),
        }))
    }

    fn new(key: KeyType, value: ValueType, height: usize) -> *mut Node<ValueType> {
        const TOP_LEVEL: usize = 31;
        let vec = (0..TOP_LEVEL + 1)
            .map(|i| AtomicPtr::new(std::ptr::null_mut()))
            .collect::<Vec<_>>();
        Box::into_raw(Box::new(Node {
            key,
            value: Some(value),
            top_level: height,
            next: vec.try_into().expect("Cannot convert to array"),
        }))
    }
}

struct SkipList<ValueType>
where
    ValueType: Clone,
{
    head: *mut Node<ValueType>,
    tail: *mut Node<ValueType>,
}

impl<ValueType> SkipList<ValueType>
where
    ValueType: Clone,
{
    const MAX_LEVEL: u64 = 31;

    pub fn new() -> SkipList<ValueType> {
        let head = Node::new_sentinel(0);
        let tail = Node::new_sentinel(u64::MAX);
        unsafe {
            for i in 0..head.as_ref().unwrap().next.len() {
                head.as_mut().unwrap().next[i] = AtomicPtr::new(tail);
            }
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
                debug_assert!(top_level <= Self::MAX_LEVEL as u64);
                let mut new_node = Node::new(key, value.clone(), top_level as usize);
                for level in 0..top_level + 1 {
                    let succ = result.succs[level as usize];
                    unsafe {
                        new_node.as_ref().unwrap().next[level as usize]
                            .store(succ, Ordering::SeqCst);
                    }
                }
                let pred = result.preds[bottom_level as usize];
                let succ = result.succs[bottom_level as usize];
                unsafe {
                    if pred.as_mut().unwrap().next[bottom_level as usize]
                        .compare_exchange(succ, new_node, Ordering::SeqCst, Ordering::SeqCst)
                        .is_err()
                    {
                        println!("Failed to replace node");
                        continue;
                    }
                }
                let mut result = result;
                for level in bottom_level + 1..=top_level {
                    loop {
                        let pred = result.preds[level as usize];
                        let succ = result.succs[level as usize];
                        unsafe {
                            if pred.as_mut().unwrap().next[level as usize]
                                .compare_exchange(
                                    succ,
                                    new_node,
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
                        let composite =
                            node_to_remove.as_ref().unwrap().next[level].load(Ordering::SeqCst);
                        succ = get_node(composite);
                        marked = get_marker(composite);
                        while !marked {
                            let _ = node_to_remove.as_ref().unwrap().next[level].compare_exchange(
                                succ,
                                add_marker(succ, true),
                                Ordering::SeqCst,
                                Ordering::SeqCst,
                            );
                            let composite =
                                node_to_remove.as_ref().unwrap().next[level].load(Ordering::SeqCst);
                            succ = get_node(composite);
                            marked = get_marker(composite);
                        }
                    }
                }
                let mut marked;
                unsafe {
                    let composite =
                        node_to_remove.as_ref().unwrap().next[BOTTOM_LEVEL].load(Ordering::SeqCst);
                    succ = get_node(composite);
                    marked = get_marker(composite);
                }
                loop {
                    let exchange_result;
                    unsafe {
                        exchange_result = node_to_remove.as_ref().unwrap().next[BOTTOM_LEVEL]
                            .compare_exchange(
                                succ,
                                add_marker(succ, true),
                                Ordering::SeqCst,
                                Ordering::SeqCst,
                            );
                        succ = get_node(
                            result.succs[BOTTOM_LEVEL].as_ref().unwrap().next[BOTTOM_LEVEL]
                                .load(Ordering::SeqCst),
                        );
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
        let mut preds = vec![std::ptr::null_mut(); top_level as usize + 1];
        let mut succs = vec![std::ptr::null_mut(); top_level as usize + 1];
        let mut to_be_freed = HashSet::new();
        'retry: loop {
            pred = self.head;
            for lvl in (bottom_level..=top_level).rev() {
                unsafe {
                    let composite =
                        pred.as_ref().unwrap().next[lvl as usize].load(Ordering::SeqCst);
                    curr = get_node(composite);
                }
                loop {
                    unsafe {
                        let composite =
                            curr.as_ref().unwrap().next[lvl as usize].load(Ordering::SeqCst);
                        succ = get_node(composite);
                        marked = get_marker(composite);
                    }
                    while marked {
                        unsafe {
                            to_be_freed.insert(curr);
                            snip = pred.as_mut().unwrap().next[lvl as usize].compare_exchange(
                                curr,
                                succ,
                                Ordering::SeqCst,
                                Ordering::SeqCst,
                            )
                        };
                        if snip.is_err() {
                            continue 'retry;
                        }
                        unsafe {
                            let composite =
                                pred.as_ref().unwrap().next[lvl as usize].load(Ordering::SeqCst);
                            debug_assert!(composite != std::ptr::null_mut());
                            curr = get_node(composite);
                            let composite = curr
                                .as_ref()
                                .expect(
                                    format!(
                                        "succ is null for key: {} where pred's key is {:?} where composite is {:?} and level is {}",
                                        key,
                                        pred.as_ref().unwrap().key,
                                        composite,
                                        lvl
                                    )
                                        .as_str(),
                                )
                                .next[lvl as usize]
                                .load(Ordering::SeqCst);
                            marked = get_marker(composite);
                            succ = get_node(composite);
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
                preds[lvl as usize] = pred;
                succs[lvl as usize] = curr;
            }
            to_be_freed.into_iter().for_each(|node| unsafe {
                let _ = Box::from_raw(node);
            });
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
