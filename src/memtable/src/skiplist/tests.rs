#[cfg(test)]
mod tests {
    use super::super::HazarPointerRecord;

    use crate::skiplist::SkipList;
    use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_skiplist() {
        let skiplist = SkipList::new();
        let hp_record = HazarPointerRecord::allocate_hp_record(
            Arc::clone(&skiplist.hazard_pointer_head),
            Arc::clone(&skiplist.max_hazard_point_count),
            5,
        );
        skiplist.add(0, "a", hp_record);
        skiplist.add(1, "b", hp_record);
        skiplist.add(2, "c", hp_record);
        skiplist.add(3, "d", hp_record);
        skiplist.add(4, "e", hp_record);
        skiplist.add(5, "f", hp_record);
        skiplist.add(6, "g", hp_record);
        skiplist.add(7, "h", hp_record);
        skiplist.add(8, "i", hp_record);
        skiplist.add(9, "j", hp_record);

        let result = skiplist.find(0, hp_record);
        assert!(result.success);
        let result = skiplist.find(1, hp_record);
        assert!(result.success);
        let result = skiplist.find(2, hp_record);
        assert!(result.success);
        let result = skiplist.find(3, hp_record);
        assert!(result.success);
        let result = skiplist.find(4, hp_record);
        assert!(result.success);
        let result = skiplist.find(5, hp_record);
        assert!(result.success);
        let result = skiplist.find(6, hp_record);
        assert!(result.success);
        let result = skiplist.find(7, hp_record);
        assert!(result.success);
        let result = skiplist.find(8, hp_record);
        assert!(result.success);
        let result = skiplist.find(9, hp_record);
        assert!(result.success);
    }

    #[test]
    fn test_skiplist_remove() {
        let skiplist = SkipList::new();

        let hp_record = HazarPointerRecord::allocate_hp_record(
            Arc::clone(&skiplist.hazard_pointer_head),
            Arc::clone(&skiplist.max_hazard_point_count),
            5,
        );

        skiplist.add(0, "a", hp_record);
        skiplist.add(1, "b", hp_record);
        skiplist.add(2, "c", hp_record);
        skiplist.add(3, "d", hp_record);
        skiplist.add(4, "e", hp_record);
        skiplist.add(5, "f", hp_record);
        skiplist.add(6, "g", hp_record);
        skiplist.add(7, "h", hp_record);
        skiplist.add(8, "i", hp_record);
        skiplist.add(9, "j", hp_record);

        let success = skiplist.remove(0, hp_record);
        assert!(success);
        let result = skiplist.find(0, hp_record);
        assert!(!result.success);
        let success = skiplist.remove(1, hp_record);
        assert!(success);
        let result = skiplist.find(1, hp_record);
        assert!(!result.success);
        let success = skiplist.remove(2, hp_record);
        assert!(success);
        let result = skiplist.find(2, hp_record);
        assert!(!result.success);
        let success = skiplist.remove(3, hp_record);
        assert!(success);
        let result = skiplist.find(3, hp_record);
        assert!(!result.success);
    }

    #[test]
    fn test_skiplist_parallel_remove() {
        let skiplist = SkipList::new();
        let head = skiplist.hazard_pointer_head.clone();
        let total_hptr_count = skiplist.max_hazard_point_count.clone();

        let hp_record = HazarPointerRecord::allocate_hp_record(
            Arc::clone(&skiplist.hazard_pointer_head),
            Arc::clone(&skiplist.max_hazard_point_count),
            5,
        );

        skiplist.add(0, "a", hp_record);
        skiplist.add(1, "b", hp_record);
        skiplist.add(2, "c", hp_record);
        skiplist.add(3, "d", hp_record);
        skiplist.add(4, "e", hp_record);
        skiplist.add(5, "f", hp_record);
        skiplist.add(6, "g", hp_record);
        skiplist.add(7, "h", hp_record);
        skiplist.add(8, "i", hp_record);
        skiplist.add(9, "j", hp_record);

        thread::scope(|s| {

            let handle = thread::Builder::new()
                .name("remove_0".into())
                .spawn_scoped(s, || {
                    let hp_record = HazarPointerRecord::allocate_hp_record(
                        Arc::clone(&head),
                        Arc::clone(&total_hptr_count),
                        5,
                    );
                    let success = skiplist.remove(0, hp_record);
                    assert!(success);
                });

            let handle = thread::Builder::new()
                .name("remove_1".into())
                .spawn_scoped(s, || {
                    let hp_record = HazarPointerRecord::allocate_hp_record(
                        Arc::clone(&head),
                        Arc::clone(&total_hptr_count),
                        5,
                    );
                    let success = skiplist.remove(1, hp_record);
                    assert!(success);
                });

            thread::Builder::new()
                .name("remove_2".into())
                .spawn_scoped(s, || {
                    let hp_record = HazarPointerRecord::allocate_hp_record(
                        Arc::clone(&head),
                        Arc::clone(&total_hptr_count),
                        5,
                    );
                    let success = skiplist.remove(2, hp_record);
                    assert!(success);
                })
                .unwrap();

            thread::Builder::new()
                .name("remove_3".into())
                .spawn_scoped(s, || {
                    let hp_record = HazarPointerRecord::allocate_hp_record(
                        Arc::clone(&head),
                        Arc::clone(&total_hptr_count),
                        5,
                    );
                    let success = skiplist.remove(3, hp_record);
                    assert!(success);
                })
                .unwrap();
        });
    }
}
