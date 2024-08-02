mod rbtree;
mod skiplist;
mod util;

trait Memtable {
    fn get(&self, key: &str) -> Option<String>;
    fn put(&mut self, key: &str, val: &str);
    fn delete(&mut self, key: &str);
}
