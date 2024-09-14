
#[cfg(test)]
mod tests {
    
    
    use std::sync::Arc;
    use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};
    use std::thread;
    use crate::memory_management::hazard_pointers::HazarPointerRecord;

    fn free_hp_records<T>(rec: *mut HazarPointerRecord<T>) {
        let mut ptr = rec;
        while !ptr.is_null() {
            let boxed_rec = unsafe { Box::from_raw(ptr) };
            ptr = boxed_rec.next;
        }
    }

    #[test]
    fn test_creation_of_head_hazard_pointer_record() {
        let total_hp_count = Arc::new(AtomicU32::new(0));
        let head = Arc::new(AtomicPtr::new(std::ptr::null_mut()));

        let hp_record: *mut HazarPointerRecord<i32> = HazarPointerRecord::allocate_hp_record(
            Arc::clone(&head),
            Arc::clone(&total_hp_count),
            5,
        );
        assert_ne!(hp_record, std::ptr::null_mut());
        assert_eq!((*head).load(Ordering::SeqCst), hp_record);
        assert_eq!(5, (*total_hp_count).load(Ordering::SeqCst));

        free_hp_records(head.load(Ordering::SeqCst));
    }

    #[test]
    fn test_creation_of_two_hazard_pointer_record() {
        let total_hp_count = Arc::new(AtomicU32::new(0));
        let head = Arc::new(AtomicPtr::new(std::ptr::null_mut()));

        let hp_record: *mut HazarPointerRecord<i32> =
            HazarPointerRecord::allocate_hp_record(head.clone(), total_hp_count.clone(), 5);

        let hp_record: *mut HazarPointerRecord<i32> =
            HazarPointerRecord::allocate_hp_record(head.clone(), total_hp_count.clone(), 5);

        assert_ne!(hp_record, std::ptr::null_mut());
        assert_eq!((*head).load(Ordering::SeqCst), hp_record);
        assert_eq!(10, (*total_hp_count).load(Ordering::SeqCst));

        free_hp_records(head.load(Ordering::SeqCst));
    }

    #[test]
    fn test_parallel_creation_of_two_hazard_pointer_records() {
        let total_hp_count = Arc::new(AtomicU32::new(0));
        let head = Arc::new(AtomicPtr::new(std::ptr::null_mut()));

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

        free_hp_records(head.load(Ordering::SeqCst));
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
        let total_hp_count = Arc::new(AtomicU32::new(0));
        let head: Arc<AtomicPtr<HazarPointerRecord<i32>>> =
            Arc::new(AtomicPtr::new(std::ptr::null_mut()));

        create_hp_record_in_parallel(head.clone(), total_hp_count.clone(), 10);

        let (h1, h2) = (Box::into_raw(Box::new(10)), Box::into_raw(Box::new(10)));
        unsafe {
            let hazard_record = (*head).load(Ordering::SeqCst);
            (*hazard_record).hazard_pointers[0] = h1;
            (*hazard_record).hazard_pointers[1] = h2;
        }

        HazarPointerRecord::retire_hp_record((*head).load(Ordering::SeqCst));

        unsafe {
            assert!(!(*(*head).load(Ordering::SeqCst))
                .active
                .load(Ordering::SeqCst));
            assert_eq!(
                (*(*head).load(Ordering::SeqCst)).hazard_pointers[0],
                std::ptr::null_mut()
            );
            assert_eq!(
                (*(*head).load(Ordering::SeqCst)).hazard_pointers[1],
                std::ptr::null_mut()
            );
        }

        unsafe {
            drop(Box::from_raw(h1));
            drop(Box::from_raw(h2));
        }

        free_hp_records(head.load(Ordering::SeqCst));
    }

    #[test]
    fn test_retire_node_should_not_raise_exception() {
        let total_hp_count = Arc::new(AtomicU32::new(0));
        let head: Arc<AtomicPtr<HazarPointerRecord<i32>>> =
            Arc::new(AtomicPtr::new(std::ptr::null_mut()));

        create_hp_record_in_parallel(head.clone(), total_hp_count.clone(), 10);

        let node = Box::into_raw(Box::new(10));
        unsafe {
            HazarPointerRecord::retire_node(
                (*head).load(Ordering::SeqCst),
                node,
                (*head).load(Ordering::SeqCst),
                0,
            );
        }

        free_hp_records(head.load(Ordering::SeqCst));
    }

    #[test]
    fn test_node_pointed_by_hazard_pointer_should_not_be_freed() {
        let total_hp_count = Arc::new(AtomicU32::new(0));
        let head: Arc<AtomicPtr<HazarPointerRecord<i32>>> =
            Arc::new(AtomicPtr::new(std::ptr::null_mut()));

        create_hp_record_in_parallel(head.clone(), total_hp_count.clone(), 5);

        let node = Box::into_raw(Box::new(10));
        let mut hp_record = head.load(Ordering::SeqCst);
        for i in 0..4 {
            unsafe {
                hp_record = (*hp_record).next;
            }
        }

        unsafe {
            (*hp_record).hazard_pointers[0] = node;
        }
        HazarPointerRecord::retire_node(hp_record, node, (*head).load(Ordering::SeqCst), 0);
        unsafe {
            drop(Box::from_raw(node));
        }

        free_hp_records(head.load(Ordering::SeqCst));
    }
}
