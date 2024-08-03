mod node;
mod tests;
mod find_result;

use crate::util::generate_random_lvl;
use node::Node;
use std::borrow::Borrow;
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::{
    convert::TryInto,
    sync::atomic::{AtomicPtr, AtomicUsize},
};
use std::{ptr, result};
use find_result::FindResult;

type KeyType = u64;

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
            let success;
            unsafe {
                success = curr.as_ref().unwrap().key == key;
            }
            return FindResult {
                success,
                preds,
                succs,
            };
        }
    }
}

