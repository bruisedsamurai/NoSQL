#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_creation_of_head_hazard_pointer_record() {
        let mut total_hp_count = Arc::new(AtomicU32::new(0));
        let mut head = Arc::new(AtomicPtr::new(std::ptr::null_mut()));

        let hp_record: *mut HazarPointerRecord<i32> = HazarPointerRecord::allocate_hp_record(
            Arc::clone(&head),
            Arc::clone(&total_hp_count),
            5,
        );
        assert_ne!(hp_record, std::ptr::null_mut());
        assert_eq!((*head).load(Ordering::SeqCst), hp_record);
        assert_eq!(5, (*total_hp_count).load(Ordering::SeqCst));
    }

    #[test]
    fn test_creation_of_two_hazard_pointer_record() {
        let mut total_hp_count = Arc::new(AtomicU32::new(0));
        let mut head = Arc::new(AtomicPtr::new(std::ptr::null_mut()));

        let hp_record: *mut HazarPointerRecord<i32> =
            HazarPointerRecord::allocate_hp_record(head.clone(), total_hp_count.clone(), 5);

        let hp_record: *mut HazarPointerRecord<i32> =
            HazarPointerRecord::allocate_hp_record(head.clone(), total_hp_count.clone(), 5);

        assert_ne!(hp_record, std::ptr::null_mut());
        assert_eq!((*head).load(Ordering::SeqCst), hp_record);
        assert_eq!(10, (*total_hp_count).load(Ordering::SeqCst));
    }

    #[test]
    fn test_parallel_creation_of_two_hazard_pointer_records() {
        let mut total_hp_count = Arc::new(AtomicU32::new(0));
        let mut head = Arc::new(AtomicPtr::new(std::ptr::null_mut()));

        let mut handles = vec![];
        for i in 0..2 {
            let handle = thread::spawn({
                let cloned_total_hp_count = Arc::clone(&total_hp_count);
                let cloned_head = Arc::clone(&head);

                move || {
                    let hp_record: *mut HazarPointerRecord<i32> =
                        HazarPointerRecord::allocate_hp_record(
                            cloned_head,
                            cloned_total_hp_count,
                            5,
                        );
                    assert_ne!(hp_record, std::ptr::null_mut());
                }
            });
            handles.push(handle);
        }
        handles.into_iter().for_each(|h| h.join().unwrap());

        let mut record = (*head).load(Ordering::SeqCst);
        let mut count = 0;
        while record != std::ptr::null_mut() {
            count += 1;
            unsafe {
                assert!((*record).active.load(Ordering::SeqCst));
                record = (*record).next;
            }
        }

        assert_eq!(10, (*total_hp_count).load(Ordering::SeqCst));
        assert_ne!((*head).load(Ordering::SeqCst), std::ptr::null_mut());
        assert_eq!(count, 2);
    }

    fn create_hp_record_in_parallel<T>(
        head: Arc<AtomicPtr<HazarPointerRecord<T>>>,
        total_hp_count: Arc<AtomicU32>,
        count: u32,
    ) {
        thread::scope(|s| {
            for i in 0..count {
                let handle = s.spawn({
                    let cloned_total_hp_count = Arc::clone(&total_hp_count);
                    let cloned_head = Arc::clone(&head);

                    move || {
                        let hp_record: *mut HazarPointerRecord<T> =
                            HazarPointerRecord::allocate_hp_record(
                                cloned_head,
                                cloned_total_hp_count,
                                5,
                            );
                        assert_ne!(hp_record, std::ptr::null_mut());
                    }
                });
            }
        })
    }

    #[test]
    fn test_retiring_node_marks_it_inactive() {
        let mut total_hp_count = Arc::new(AtomicU32::new(0));
        let mut head: Arc<AtomicPtr<HazarPointerRecord<i32>>> =
            Arc::new(AtomicPtr::new(std::ptr::null_mut()));

        create_hp_record_in_parallel(head.clone(), total_hp_count.clone(), 2);

        HazarPointerRecord::retire_hp_record((*head).load(Ordering::SeqCst));

        unsafe {
            assert!(!(*(*head).load(Ordering::SeqCst))
                .active
                .load(Ordering::SeqCst));
        }
    }
}

// Michael, Maged M. "Hazard pointers: Safe memory reclamation for lock-free objects."
// IEEE Transactions on Parallel and Distributed Systems 15, no. 6 (2004): 491-504.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};
use std::sync::Arc;

pub struct HazarPointerRecord<T> {
    pub hazar_pointers: Vec<*mut T>,
    pub next: *mut HazarPointerRecord<T>,
    pub active: AtomicBool,
    pub r_list: HashSet<*mut T>,
    pub r_count: usize,
}

impl<T> HazarPointerRecord<T> {
    pub fn allocate_hp_record(
        head: Arc<AtomicPtr<HazarPointerRecord<T>>>,
        total_hp_count: Arc<AtomicU32>,
        per_record_hp_count: u32,
    ) -> *mut HazarPointerRecord<T> {
        let mut hp_record: *mut HazarPointerRecord<T>;
        unsafe {
            hp_record = (*head).load(Ordering::SeqCst);
        }
        while hp_record != std::ptr::null_mut() {
            unsafe {
                if (*hp_record).active.load(Ordering::SeqCst) {
                    hp_record = (*hp_record).next;
                    continue;
                }
                if !(*hp_record)
                    .active
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    hp_record = (*hp_record).next;
                    continue;
                }
            }
            return hp_record;
        }

        loop {
            unsafe {
                let oldcount = (*total_hp_count).load(Ordering::SeqCst);
                if (*total_hp_count)
                    .compare_exchange(
                        oldcount,
                        oldcount + per_record_hp_count,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    )
                    .is_ok()
                {
                    break;
                }
            }
        }

        //cursed code
        let hprec = Box::into_raw(Box::new(HazarPointerRecord {
            hazar_pointers: vec![std::ptr::null_mut(); per_record_hp_count as usize],
            next: std::ptr::null_mut(),
            active: AtomicBool::new(true),
            r_list: HashSet::new(),
            r_count: 0,
        }));

        loop {
            unsafe {
                let old_head = (*head).load(Ordering::SeqCst);
                (*hprec).next = old_head;
                if let Ok(ptr) =
                    (*head).compare_exchange(old_head, hprec, Ordering::SeqCst, Ordering::SeqCst)
                {
                    // let _ = Box::from_raw(ptr);
                    break;
                }
            }
        }
        unsafe { hprec }
    }

    pub fn retire_hp_record(rec_node: *mut HazarPointerRecord<T>) {
        let hp_length;
        let hp_list: &mut Vec<*mut T>;
        unsafe {
            hp_length = (*rec_node).hazar_pointers.len();
            hp_list = (*rec_node).hazar_pointers.as_mut();
        }
        for i in 0..hp_length {
            hp_list[i] = std::ptr::null_mut();
        }
        unsafe {
            (*rec_node).active.store(false, Ordering::SeqCst);
        }
    }

    pub fn retire_node(
        &mut self,
        node: *mut T,
        head: *mut HazarPointerRecord<T>,
        max_hptr_count: usize,
    ) {
        self.r_list.insert(node);
        self.r_count += 1;
        if self.r_count >= max_hptr_count {
            self.scan(head);
            self.help_scan(head, max_hptr_count);
        }
    }

    pub fn help_scan(&mut self, head_hp_record: *mut HazarPointerRecord<T>, max_hptr_count: usize) {
        let mut hp_record = head_hp_record;
        while head_hp_record != std::ptr::null_mut() {
            unsafe {
                if (*hp_record).active.load(Ordering::SeqCst) {
                    continue;
                }
                if !(*hp_record)
                    .active
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    continue;
                }
                for node in (*hp_record).r_list.iter() {
                    (*hp_record).r_list.remove(node);
                    (*hp_record).r_count -= 1;
                    self.r_list.insert(*node);
                    self.r_count += 1;
                    let head = head_hp_record;
                    if self.r_count >= max_hptr_count {
                        self.scan(head);
                    }
                }
                (*hp_record).active.store(false, Ordering::SeqCst);
                hp_record = (*hp_record).next;
            }
        }
    }

    pub fn scan(&mut self, head: *mut HazarPointerRecord<T>) {
        let mut hazard_ptr_collection: HashSet<*mut T> = HashSet::new();
        let mut hp_record = head;
        while hp_record != std::ptr::null_mut() {
            unsafe {
                for h_pointer in (*hp_record).hazar_pointers.clone() {
                    if !h_pointer.is_null() {
                        hazard_ptr_collection.insert(h_pointer);
                    }
                }
                hp_record = (*hp_record).next;
            }
        }

        let iter = self.r_list.drain().collect::<Vec<*mut T>>();
        self.r_count = 0;
        for node in iter {
            if hazard_ptr_collection.contains(&node) {
                self.r_list.insert(node);
                self.r_count += 1;
            } else {
                unsafe {
                    let _ = Box::from_raw(node);
                }
            }
        }
    }
}
