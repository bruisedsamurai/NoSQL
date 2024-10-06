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
use std::collections::HashSet;
use std::ptr;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

// Reason for using pointers directly
// https://rust-unofficial.github.io/too-many-lists/fifth-stacked-borrows.html

#[inline(always)]
fn get_node<ValueType>(ptr: *mut Node<ValueType>) -> *mut Node<ValueType>
where
    ValueType: Clone,
{
    (ptr as usize & !0x1) as *mut Node<ValueType>
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
        (ptr as usize & !0x1) as *mut Node<ValueType>
    }
}

struct SkipList<ValueType>
where
    ValueType: Clone,
{
    head: *mut Node<ValueType>,
    tail: *mut Node<ValueType>,
    hazard_pointer_head: Arc<AtomicPtr<HazarPointerRecord<Node<ValueType>>>>,
    max_hazard_point_count: Arc<AtomicU32>,
}

impl<ValueType> SkipList<ValueType>
where
    ValueType: Clone,
{
    const MAX_LEVEL: u64 = Node::<ValueType>::TOP_LEVEL as u64;
    const MAX_HP: u64 = 5;

    pub fn new() -> SkipList<ValueType> {
        let head = Node::new_sentinel(i128::MIN);
        let tail = Node::new_sentinel(i128::MAX);
        let depth;
        unsafe {
            depth = (*head).next.len();
        }
        for i in 0..depth {
            unsafe {
                (*head).next[i] = AtomicPtr::new(tail);
            }
        }
        let max_hp_count = Arc::new(AtomicU32::new(0));
        let hp_head = Arc::new(AtomicPtr::new(std::ptr::null_mut()));
        HazarPointerRecord::allocate_hp_record(hp_head.clone(), max_hp_count.clone(), 5);
        SkipList {
            head,
            tail,
            hazard_pointer_head: hp_head,
            max_hazard_point_count: max_hp_count,
        }
    }

    pub fn add(
        &self,
        key: KeyType,
        value: ValueType,
        hp_record: *mut HazarPointerRecord<Node<ValueType>>,
    ) -> bool {
        let hp_record = self.enter();
        let top_level = generate_random_lvl(Self::MAX_LEVEL) as usize;
        let bottom_level = 0;
        loop {
            let result = self.find(key, hp_record);
            if result.success {
                self.exit(hp_record);
                return false;
            } else {
                debug_assert!(top_level <= Self::MAX_LEVEL as usize);
                let new_node = Node::new(key, value.clone(), top_level);
                for level in bottom_level..=top_level {
                    let succ = result.succs[level];
                    unsafe {
                        (*new_node).next[level].store(succ, Ordering::SeqCst);
                    }
                }
                let pred = result.preds[bottom_level];
                unsafe {
                    (*hp_record).hazard_pointers[0].store(pred, Ordering::SeqCst);
                }
                let succ = result.succs[bottom_level];
                unsafe {
                    (*hp_record).hazard_pointers[1].store(succ, Ordering::SeqCst);
                }
                unsafe {
                    if (*pred).next[bottom_level]
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
                        let pred = result.preds[level];
                        unsafe {
                            (*hp_record).hazard_pointers[2].store(pred, Ordering::SeqCst);
                        }
                        let succ = result.succs[level];
                        unsafe {
                            (*hp_record).hazard_pointers[3].store(succ, Ordering::SeqCst);
                        }
                        unsafe {
                            if (*pred).next[level]
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
                        result = self.find(key, hp_record);
                    }
                }
                self.exit(hp_record);
                return true;
            }
        }
    }

    pub fn remove(
        &self,
        key: KeyType,
        hp_record: *mut HazarPointerRecord<Node<ValueType>>,
    ) -> bool {
        let hp_record = self.enter();
        const BOTTOM_LEVEL: usize = 0;
        let mut succ;
        'retry: loop {
            let result = self.find(key, hp_record);
            if !result.success {
                self.exit(hp_record);
                return false;
            } else {
                let node_to_remove = result.succs[BOTTOM_LEVEL];
                unsafe {
                    (*hp_record).hazard_pointers[0].store(node_to_remove, Ordering::SeqCst);
                }
                let height;
                unsafe {
                    height = (*node_to_remove).top_level;
                }
                for level in (BOTTOM_LEVEL + 1..=height).rev() {
                    let mut marked;
                    unsafe {
                        let composite = (*node_to_remove).next[level].load(Ordering::SeqCst);
                        succ = get_node(composite);
                        (*hp_record).hazard_pointers[1].store(succ, Ordering::SeqCst);
                        marked = get_marker(composite);
                        // Keep trying to mark successor to predecessor until it's not marked
                        while !marked {
                            let _ = (*node_to_remove).next[level].compare_exchange(
                                succ,
                                add_marker(succ, true),
                                Ordering::SeqCst,
                                Ordering::SeqCst,
                            );
                            let composite = (*node_to_remove).next[level].load(Ordering::SeqCst);
                            succ = get_node(composite);
                            (*hp_record).hazard_pointers[1].store(succ, Ordering::SeqCst);
                            marked = get_marker(composite);
                        }
                    }
                }
                let mut marked;
                unsafe {
                    let composite = (*node_to_remove).next[BOTTOM_LEVEL].load(Ordering::SeqCst);
                    succ = get_node(composite);
                    (*hp_record).hazard_pointers[1].store(succ, Ordering::SeqCst);
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
                        (*hp_record).hazard_pointers[2].store(succ, Ordering::SeqCst);
                        marked = get_marker(composite);
                    }
                    // If bottom level node was marked; find it and return true else if
                    // failed and someone else marked it then return false
                    if exchange_result.is_ok() {
                        let _ = self.find(key, hp_record);
                        self.exit(hp_record);
                        return true;
                    } else if marked {
                        self.exit(hp_record);
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
    ) -> FindResult<ValueType> {
        let hazard_pointer_record = self.enter();
        const BOTTOM_LEVEL: u64 = 0;
        let top_level = Self::MAX_LEVEL;
        let mut snip;
        let mut pred: *mut Node<ValueType>;
        let mut marked: bool;
        let mut curr = ptr::null_mut();
        let mut succ;
        let mut preds = vec![std::ptr::null_mut(); top_level as usize + 1];
        let mut succs = vec![std::ptr::null_mut(); top_level as usize + 1];
        let mut free_collection = HashSet::new();
        'retry: loop {
            pred = self.head;
            unsafe {
                (*hazard_pointer_record).hazard_pointers[0].store(pred, Ordering::SeqCst);
            }
            for lvl in (BOTTOM_LEVEL..=top_level).rev() {
                unsafe {
                    let composite = (*pred).next[lvl as usize].load(Ordering::SeqCst);
                    curr = get_node(composite);
                    (*hazard_pointer_record).hazard_pointers[1].store(curr, Ordering::SeqCst);
                }
                loop {
                    unsafe {
                        let composite = (*curr).next[lvl as usize].load(Ordering::SeqCst);
                        succ = get_node(composite);
                        (*hazard_pointer_record).hazard_pointers[2].store(succ, Ordering::SeqCst);
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
                        if lvl == BOTTOM_LEVEL {
                            free_collection.insert(curr);
                        }
                        unsafe {
                            let composite = (*pred).next[lvl as usize].load(Ordering::SeqCst);
                            debug_assert!(composite != std::ptr::null_mut());
                            curr = get_node(composite);
                            (*hazard_pointer_record).hazard_pointers[1]
                                .store(curr, Ordering::SeqCst);
                            let composite = (*curr).next[lvl as usize].load(Ordering::SeqCst);
                            marked = get_marker(composite);
                            succ = get_node(composite);
                            (*hazard_pointer_record).hazard_pointers[2]
                                .store(succ, Ordering::SeqCst);
                        }
                    }
                    let curr_key = unsafe { (*curr).key };
                    if curr_key < key {
                        pred = curr;
                        curr = succ;
                    } else {
                        break;
                    }
                }
                preds[lvl as usize] = pred;
                succs[lvl as usize] = curr;
            }
            for node in free_collection.into_iter() {
                HazarPointerRecord::retire_node(
                    self.hazard_pointer_head.load(Ordering::SeqCst),
                    hazard_pointer_record,
                    node,
                    1,
                );
            }
            let success = unsafe { (*curr).key == key };
            self.exit(hazard_pointer_record);
            return FindResult {
                success,
                preds,
                succs,
            };
        }
    }

    fn enter(&self) -> *mut HazarPointerRecord<Node<ValueType>> {
        HazarPointerRecord::allocate_hp_record(
            self.hazard_pointer_head.clone(),
            self.max_hazard_point_count.clone(),
            Self::MAX_HP as u32,
        )
    }

    fn exit(&self, hp_record: *mut HazarPointerRecord<Node<ValueType>>) {
        HazarPointerRecord::retire_hp_record(hp_record);
    }
}

impl<ValueType> Drop for SkipList<ValueType>
where
    ValueType: Clone,
{
    fn drop(&mut self) {
        fn drop_hp_records<ValueType>(head: *mut HazarPointerRecord<ValueType>) {
            let mut record = head;
            while (record != std::ptr::null_mut()) {
                unsafe {
                    let temp = Box::from_raw(record);
                    record = temp.next.load(Ordering::SeqCst);
                }
            }
        }

        drop_hp_records(self.hazard_pointer_head.load(Ordering::SeqCst));

        let mut curr = self.head;
        let mut free_collection = HashSet::new();
        for lvl in (0..=Self::MAX_LEVEL).rev() {
            while curr != self.tail {
                let next;
                unsafe {
                    next = (*curr).next[lvl as usize].load(Ordering::SeqCst);
                }
                free_collection.insert(curr);
                curr = get_node(next);
            }
            free_collection.insert(curr);
            curr = self.head;
        }
        free_collection.insert(self.head);
        for node in free_collection.into_iter() {
            unsafe {
                let _ = drop(Box::from_raw(node));
            }
        }
    }
}

unsafe impl<V> Send for SkipList<V> where V: Clone {}
unsafe impl<V> Sync for SkipList<V> where V: Clone {}
