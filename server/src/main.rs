use red_black_tree::RBTree;

fn main() {
    let mut rb_tree = RBTree::new();
    rb_tree.insert("A", 1);
    println!("!{:?}", &rb_tree);
    rb_tree.insert("b", 3);
    println!("!{:?}", &rb_tree);
    rb_tree.insert("c", 3);
    println!("!{:?}", &rb_tree);
    rb_tree.insert("d", 3);
    println!("!{:?}", &rb_tree);
    rb_tree.insert("e", 3);
    println!("!{:?}", &rb_tree);
    println!("{:?}", &rb_tree.search("a"));
    println!("{:?}", &rb_tree.search(""));
    /*
    for each in rb_tree.iter() {
        println!("{:?}", each);
    }
    */
}
