#[cfg(test)]
mod tests {
    use super::*;
    use crate::skiplist::SkipList;
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
