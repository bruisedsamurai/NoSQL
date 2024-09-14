//Algorithm reference
// Herlihy, Maurice, Nir Shavit, Victor Luchangco, and Michael Spear.
// The art of multiprocessor programming. Newnes, 2020.
mod find_result;
mod node;
mod tests;

use crate::memory_management::hazard_pointers::HazarPointerRecord;
use crate::util::generate_random_lvl;
use find_result::FindResult;
use node::{KeyType, Node};
use std::ptr;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicPtr;

// Reason for using pointers directly
// https://rust-unofficial.github.io/too-many-lists/fifth-stacked-borrows.html

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
        let head = Node::new_sentinel(i128::MIN);
        let tail = Node::new_sentinel(i128::MAX);
        unsafe {
            for i in 0..(*head).next.len() {
                (*head).next[i] = AtomicPtr::new(tail);
            }
        }
        SkipList { head, tail }
    }

    pub fn add(
        &self,
        key: KeyType,
        value: ValueType,
        hp_record: *mut HazarPointerRecord<Node<ValueType>>,
        hp_head: *mut HazarPointerRecord<Node<ValueType>>,
    ) -> bool {
        let top_level = generate_random_lvl(Self::MAX_LEVEL);
        let bottom_level = 0;
        loop {
            let result = self.find(key, hp_record, hp_head);
            if result.success {
                return false;
            } else {
                debug_assert!(top_level <= Self::MAX_LEVEL);
                let new_node = Node::new(key, value.clone(), top_level as usize);
                for level in 0..top_level + 1 {
                    let succ = result.succs[level as usize];
                    unsafe {
                        (*new_node).next[level as usize].store(succ, Ordering::SeqCst);
                    }
                }
                let pred = result.preds[bottom_level as usize];
                unsafe {
                    (*hp_record).hazard_pointers[0] = pred;
                }
                let succ = result.succs[bottom_level as usize];
                unsafe {
                    (*hp_record).hazard_pointers[1] = succ;
                }
                unsafe {
                    if (*pred).next[bottom_level as usize]
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
                        unsafe {
                            (*hp_record).hazard_pointers[2] = pred;
                        }
                        let succ = result.succs[level as usize];
                        unsafe {
                            (*hp_record).hazard_pointers[3] = succ;
                        }
                        unsafe {
                            if (*pred).next[level as usize]
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
                        result = self.find(key, hp_record, hp_head);
                    }
                }
                return true;
            }
        }
    }

    pub fn remove(
        &self,
        key: KeyType,
        hp_record: *mut HazarPointerRecord<Node<ValueType>>,
        hp_head: *mut HazarPointerRecord<Node<ValueType>>,
    ) -> bool {
        const BOTTOM_LEVEL: usize = 0;
        let mut succ;
        loop {
            let result = self.find(key, hp_record, hp_head);
            if !result.success {
                return false;
            } else {
                let node_to_remove = result.succs[BOTTOM_LEVEL];
                let height;
                unsafe {
                    height = (*node_to_remove).top_level;
                }
                for level in (BOTTOM_LEVEL + 1..=height).rev() {
                    let mut marked;
                    unsafe {
                        let composite = (*node_to_remove).next[level].load(Ordering::SeqCst);
                        succ = get_node(composite);
                        (*hp_record).hazard_pointers[0] = succ;
                        marked = get_marker(composite);
                        while !marked {
                            let _ = (*node_to_remove).next[level].compare_exchange(
                                succ,
                                add_marker(succ, true),
                                Ordering::SeqCst,
                                Ordering::SeqCst,
                            );
                            let composite = (*node_to_remove).next[level].load(Ordering::SeqCst);
                            succ = get_node(composite);
                            (*hp_record).hazard_pointers[0] = succ;
                            marked = get_marker(composite);
                        }
                    }
                }
                let mut marked;
                unsafe {
                    let composite = (*node_to_remove).next[BOTTOM_LEVEL].load(Ordering::SeqCst);
                    succ = get_node(composite);
                    (*hp_record).hazard_pointers[1] = succ;
                    marked = get_marker(composite);
                }
                loop {
                    let exchange_result;
                    unsafe {
                        exchange_result = (*node_to_remove).next[BOTTOM_LEVEL].compare_exchange(
                            succ,
                            add_marker(succ, true),
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        );
                        let composite =
                            (*result.succs[BOTTOM_LEVEL]).next[BOTTOM_LEVEL].load(Ordering::SeqCst);
                        succ = get_node(composite);
                        (*hp_record).hazard_pointers[2] = succ;
                        marked = get_marker(composite);
                    }
                    if exchange_result.is_ok() {
                        let _ = self.find(key, hp_record, hp_head);
                        return true;
                    } else if marked {
                        return false;
                    }
                }
            }
        }
    }

    fn find(
        &self,
        key: KeyType,
        hazard_pointer_record: *mut HazarPointerRecord<Node<ValueType>>,
        hp_head: *mut HazarPointerRecord<Node<ValueType>>,
    ) -> FindResult<ValueType> {
        const BOTTOM_LEVEL: u64 = 0;
        let top_level = Self::MAX_LEVEL;
        let mut snip;
        let mut pred: *mut Node<ValueType>;
        let mut marked: bool;
        let mut curr = ptr::null_mut();
        let mut succ;
        let mut preds = vec![std::ptr::null_mut(); top_level as usize + 1];
        let mut succs = vec![std::ptr::null_mut(); top_level as usize + 1];
        'retry: loop {
            pred = self.head;
            unsafe {
                (*hazard_pointer_record).hazard_pointers[0] = pred;
            }
            for lvl in (BOTTOM_LEVEL..=top_level).rev() {
                unsafe {
                    let composite = (*pred).next[lvl as usize].load(Ordering::SeqCst);
                    curr = get_node(composite);
                    (*hazard_pointer_record).hazard_pointers[1] = curr;
                }
                loop {
                    unsafe {
                        let composite = (*curr).next[lvl as usize].load(Ordering::SeqCst);
                        succ = get_node(composite);
                        (*hazard_pointer_record).hazard_pointers[2] = succ;
                        marked = get_marker(composite);
                    }
                    while marked {
                        unsafe {
                            snip = (*pred).next[lvl as usize].compare_exchange(
                                curr,
                                succ,
                                Ordering::SeqCst,
                                Ordering::SeqCst,
                            )
                        };
                        if snip.is_err() {
                            continue 'retry;
                        }
                        HazarPointerRecord::retire_node(hazard_pointer_record, curr, hp_head, 1);
                        unsafe {
                            let composite = (*pred).next[lvl as usize].load(Ordering::SeqCst);
                            debug_assert!(composite != std::ptr::null_mut());
                            curr = get_node(composite);
                            (*hazard_pointer_record).hazard_pointers[1] = curr;
                            let composite = (*curr).next[lvl as usize].load(Ordering::SeqCst);
                            marked = get_marker(composite);
                            succ = get_node(composite);
                            (*hazard_pointer_record).hazard_pointers[2] = succ;
                        }
                    }
                    unsafe {
                        if (*curr).key < key {
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
            let success;
            unsafe {
                success = (*curr).key == key;
            }
            return FindResult {
                success,
                preds,
                succs,
            };
        }
    }
}

impl<ValueType> Drop for SkipList<ValueType>
where
    ValueType: Clone,
{
    fn drop(&mut self) {
        unsafe {
            let mut curr = self.head;
            while curr != self.tail {
                let next = (*curr).next[0].load(Ordering::SeqCst);
                let node = Box::from_raw(curr);
                curr = get_node(next);
            }
            let node = Box::from_raw(curr);
        }
    }
}

unsafe impl<V> Send for SkipList<V> where V: Clone {}
unsafe impl<V> Sync for SkipList<V> where V: Clone {}
