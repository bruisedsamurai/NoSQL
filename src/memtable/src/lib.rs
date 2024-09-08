mod rbtree;
mod skiplist;
mod util;
mod memory_management;

trait Memtable {
    fn get(&self, key: &str) -> Option<String>;
    fn put(&mut self, key: &str, val: &str);
    fn delete(&mut self, key: &str);
}
