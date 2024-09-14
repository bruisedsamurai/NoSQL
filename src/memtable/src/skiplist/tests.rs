﻿#[cfg(test)]
mod tests {
    use super::super::HazarPointerRecord;
    use super::*;
    use crate::skiplist::SkipList;
    use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_skiplist() {
        let mut skiplist = SkipList::new();
        let mut head = Arc::new(AtomicPtr::new(std::ptr::null_mut()));
        let mut total_hptr_count = Arc::new(AtomicU32::new(0));
        let hp_record = HazarPointerRecord::allocate_hp_record(
            Arc::clone(&head),
            Arc::clone(&total_hptr_count),
            5,
        );
        skiplist.add(0, "a", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(1, "b", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(2, "c", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(3, "d", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(4, "e", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(5, "f", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(6, "g", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(7, "h", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(8, "i", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(9, "j", hp_record, head.load(Ordering::SeqCst));

        let result = skiplist.find(0, hp_record, head.load(Ordering::SeqCst));
        assert_eq!(result.success, true);
        let result = skiplist.find(1, hp_record, head.load(Ordering::SeqCst));
        assert!(result.success);
        let result = skiplist.find(2, hp_record, head.load(Ordering::SeqCst));
        assert!(result.success);
        let result = skiplist.find(3, hp_record, head.load(Ordering::SeqCst));
        assert!(result.success);
        let result = skiplist.find(4, hp_record, head.load(Ordering::SeqCst));
        assert!(result.success);
        let result = skiplist.find(5, hp_record, head.load(Ordering::SeqCst));
        assert!(result.success);
        let result = skiplist.find(6, hp_record, head.load(Ordering::SeqCst));
        assert!(result.success);
        let result = skiplist.find(7, hp_record, head.load(Ordering::SeqCst));
        assert!(result.success);
        let result = skiplist.find(8, hp_record, head.load(Ordering::SeqCst));
        assert!(result.success);
        let result = skiplist.find(9, hp_record, head.load(Ordering::SeqCst));
        assert!(result.success);
    }

    #[test]
    fn test_skiplist_remove() {
        let mut skiplist = SkipList::new();

        let mut head = Arc::new(AtomicPtr::new(std::ptr::null_mut()));
        let mut total_hptr_count = Arc::new(AtomicU32::new(0));
        let hp_record = HazarPointerRecord::allocate_hp_record(
            Arc::clone(&head),
            Arc::clone(&total_hptr_count),
            5,
        );

        skiplist.add(0, "a", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(1, "b", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(2, "c", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(3, "d", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(4, "e", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(5, "f", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(6, "g", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(7, "h", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(8, "i", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(9, "j", hp_record, head.load(Ordering::SeqCst));

        let success = skiplist.remove(0, hp_record, head.load(Ordering::SeqCst));
        assert!(success);
        let result = skiplist.find(0, hp_record, head.load(Ordering::SeqCst));
        assert!(!result.success);
        let success = skiplist.remove(1, hp_record, head.load(Ordering::SeqCst));
        assert!(success);
        let result = skiplist.find(1, hp_record, head.load(Ordering::SeqCst));
        assert!(!result.success);
        let success = skiplist.remove(2, hp_record, head.load(Ordering::SeqCst));
        assert!(success);
        let result = skiplist.find(2, hp_record, head.load(Ordering::SeqCst));
        assert!(!result.success);
        let success = skiplist.remove(3, hp_record, head.load(Ordering::SeqCst));
        assert!(success);
        let result = skiplist.find(3, hp_record, head.load(Ordering::SeqCst));
        assert!(!result.success);
    }

    #[test]
    fn test_skiplist_parallel_remove() {
        let mut skiplist = SkipList::new();

        let mut head = Arc::new(AtomicPtr::new(std::ptr::null_mut()));
        let mut total_hptr_count = Arc::new(AtomicU32::new(0));
        let hp_record = HazarPointerRecord::allocate_hp_record(
            Arc::clone(&head),
            Arc::clone(&total_hptr_count),
            5,
        );

        skiplist.add(0, "a", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(1, "b", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(2, "c", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(3, "d", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(4, "e", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(5, "f", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(6, "g", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(7, "h", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(8, "i", hp_record, head.load(Ordering::SeqCst));
        skiplist.add(9, "j", hp_record, head.load(Ordering::SeqCst));

        thread::scope(|s| {
            let arc_head = Arc::clone(&head);
            let arc_total_hptr_count = Arc::clone(&total_hptr_count);

            s.spawn(|| {
                let hp_record = HazarPointerRecord::allocate_hp_record(
                    Arc::clone(&head),
                    Arc::clone(&total_hptr_count),
                    5,
                );
                let success = skiplist.remove(0, hp_record, head.load(Ordering::SeqCst));
                assert!(success);
            });

            let handle = s.spawn(|| {
                let hp_record = HazarPointerRecord::allocate_hp_record(
                    Arc::clone(&head),
                    Arc::clone(&total_hptr_count),
                    5,
                );
                let success = skiplist.remove(1, hp_record, head.load(Ordering::SeqCst));
                assert!(success);
            });

            handle.join().unwrap();

            s.spawn(|| {
                let hp_record = HazarPointerRecord::allocate_hp_record(
                    Arc::clone(&head),
                    Arc::clone(&total_hptr_count),
                    5,
                );
                let success = skiplist.remove(2, hp_record, head.load(Ordering::SeqCst));
                assert!(success);
            });

            s.spawn(|| {
                let hp_record = HazarPointerRecord::allocate_hp_record(
                    Arc::clone(&head),
                    Arc::clone(&total_hptr_count),
                    5,
                );
                let success = skiplist.remove(3, hp_record, head.load(Ordering::SeqCst));
                assert!(success);
            });
        });
    }
}
