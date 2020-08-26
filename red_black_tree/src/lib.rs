#[cfg(test)]
mod tests {
    use super::*;
    use rand::distributions::Alphanumeric;
    use rand::Rng;

    #[test]
    fn test_node_color_update() {
        let mut node = Node::new(String::from("a"), 2, Color::Red, None, None, None);
        node.update_color(Color::Black);
        assert_eq!(Color::Black, node.get_color());
    }

    #[test]
    fn test_get_left_child() {
        let node_1 = Node::new(String::from("a"), 2, Color::Red, None, None, None);
        let node_1_rc = Rc::new(RefCell::new(node_1));
        let node_2 = Node::new(
            String::from("a"),
            2,
            Color::Red,
            Some(Rc::clone(&node_1_rc)),
            None,
            None,
        );

        assert!(Rc::ptr_eq(&node_1_rc, &node_2.get_left_child().unwrap()))
    }

    #[test]
    fn test_get_right_child() {
        let node_1 = Node::new(String::from("a"), 2, Color::Red, None, None, None);
        let node_1_rc = Rc::new(RefCell::new(node_1));
        let node_2 = Node::new(
            String::from("a"),
            2,
            Color::Red,
            None,
            Some(Rc::clone(&node_1_rc)),
            None,
        );

        assert!(Rc::ptr_eq(&node_1_rc, &node_2.get_right_child().unwrap()))
    }

    #[test]
    fn test_get_parent() {
        let node_1 = Node::new(String::from("a"), 2, Color::Red, None, None, None);
        let node_1_rc = Rc::new(RefCell::new(node_1));
        let node_2 = Node::new(
            String::from("a"),
            2,
            Color::Red,
            None,
            None,
            Some(Rc::downgrade(&node_1_rc)),
        );

        assert!(Rc::ptr_eq(&node_1_rc, &node_2.get_parent().unwrap()))
    }

    #[test]
    fn test_tree_insertion_and_search() {
        let sample = |_| {
            rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(20)
                .collect::<String>()
        };
        let sample_vec: Vec<String> = [0..10].iter().map(sample).collect();

        let mut rb_tree = RBTree::new();

        for key in &sample_vec {
            rb_tree.insert(key, 1);
        }

        for key in &sample_vec {
            assert!(
                &rb_tree.search(key).unwrap().0 == key,
                format!("Did not find key: {}", key)
            );
        }
    }

    #[test]
    fn test_tree_order() {
        let arr: [i32; 20] = rand::random();
        let char_arr: [char; 20] = rand::random();

        let zip_arr: Vec<(&char, &i32)> = char_arr.iter().zip(arr.iter()).collect();

        let mut tree = RBTree::new();

        for (ch, val) in zip_arr.iter() {
            tree.insert(&ch.to_string(), **val);
        }

        let mut char_arr = char_arr;
        char_arr.sort();

        let tree_vec: Vec<(String, i32)> = tree.iter().collect();

        let mut i1 = 0;
        let mut i2 = 0;

        while i1 != char_arr.len() && i2 != tree_vec.len() {
            assert_eq!(char_arr[i1].to_string(), tree_vec[i2].0);
            i1 += 1;
            i2 += 1;
        }
    }
}

use std::cell::RefCell;
use std::rc::{Rc, Weak};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Color {
    Red,
    Black,
}

#[derive(Debug)]
pub struct Node {
    pub key: String,
    pub value: i32,
    color: Color,
    pub left: Option<Rc<RefCell<Node>>>,
    pub right: Option<Rc<RefCell<Node>>>,
    pub parent: Option<Weak<RefCell<Node>>>,
}

impl Node {
    pub fn new(
        key: String,
        value: i32,
        color: Color,
        left: Option<Rc<RefCell<Node>>>,
        right: Option<Rc<RefCell<Node>>>,
        parent: Option<Weak<RefCell<Node>>>,
    ) -> Node {
        Node {
            key,
            value,
            color,
            left,
            right,
            parent,
        }
    }

    pub fn get_color(&self) -> Color {
        self.color
    }

    pub fn update_color(&mut self, new_color: Color) {
        self.color = new_color;
    }

    pub fn get_left_child(&self) -> Option<Rc<RefCell<Node>>> {
        if let Some(left) = &self.left {
            Some(Rc::clone(left))
        } else {
            None
        }
    }

    pub fn get_right_child(&self) -> Option<Rc<RefCell<Node>>> {
        if let Some(right) = &self.right {
            Some(Rc::clone(right))
        } else {
            None
        }
    }

    pub fn get_parent(&self) -> Option<Rc<RefCell<Node>>> {
        if let Some(parent) = &self.parent {
            parent.upgrade()
        } else {
            None
        }
    }
}

#[derive(Debug, Default)]
pub struct RBTree {
    root: Option<Rc<RefCell<Node>>>,
}

impl RBTree {
    pub fn new() -> RBTree {
        RBTree { root: None }
    }

    pub fn search(&self, key: &str) -> Option<(String, i32)> {
        let mut iter: Option<Rc<RefCell<Node>>> = self.root.as_ref().cloned();

        while let Some(iter_node) = iter {
            if key == iter_node.borrow().key {
                return Some((String::from(key), iter_node.borrow().value));
            } else if key > &iter_node.borrow().key {
                iter = iter_node.borrow().get_right_child();
            } else {
                iter = iter_node.borrow().get_left_child();
            }
        }
        None
    }

    pub fn insert(&mut self, key: &str, val: i32) {
        let mut leaf_node: Option<Rc<RefCell<Node>>> = None;
        let mut iter: Option<Rc<RefCell<Node>>> = self.root.as_ref().cloned();

        while let Some(iter_node) = iter {
            leaf_node = Some(Rc::clone(&iter_node));
            if key < &iter_node.borrow().key {
                iter = iter_node.borrow().get_left_child();
            }
            // We keep keys with equal value to the right
            else {
                iter = iter_node.borrow().get_right_child();
            }
        }

        let new_node = Node::new(String::from(key), val, Color::Red, None, None, None);
        let new_node_rc = Rc::new(RefCell::new(new_node));

        if let Some(leaf) = leaf_node.as_ref() {
            if new_node_rc.borrow().key < leaf.borrow().key {
                leaf.borrow_mut().left = Some(Rc::clone(&new_node_rc));
            } else {
                leaf.borrow_mut().right = Some(Rc::clone(&new_node_rc));
            }
            new_node_rc.borrow_mut().parent = Some(Rc::downgrade(leaf));
        } else {
            self.root = Some(Rc::clone(&new_node_rc));
        }

        self.insert_fixup(new_node_rc);
    }

    fn insert_fixup(&mut self, new_node: Rc<RefCell<Node>>) {
        let mut curr_node = new_node;
        while curr_node.borrow().get_parent().is_some()
            && curr_node
                .borrow()
                .get_parent()
                .unwrap()
                .borrow()
                .get_color()
                == Color::Red
        {
            let curr_node_p = curr_node.borrow().get_parent().expect("No parent found");
            let curr_node_gp = curr_node_p
                .borrow()
                .get_parent()
                .expect("No grand parent found"); //grand parent is gauranteed to exist because we have parent's color red but root always have black color
            if matches!(curr_node_gp.borrow().get_left_child(), Some(_))
                && Rc::ptr_eq(
                    &curr_node_p,
                    &curr_node_gp
                        .borrow()
                        .get_left_child()
                        .expect("Left child of grandparent does not exist"),
                )
            {
                let uncle_option: Option<Rc<RefCell<Node>>> =
                    curr_node_gp.borrow().get_right_child();
                match uncle_option {
                    Some(uncle) if matches!(uncle.borrow().get_color(), Color::Red) => {
                        curr_node_p.borrow_mut().update_color(Color::Black);
                        uncle.borrow_mut().update_color(Color::Black);
                        curr_node_gp.borrow_mut().update_color(Color::Red);
                    }
                    _ => {
                        if matches!(curr_node_p.borrow().get_right_child(), Some(_))
                            && Rc::ptr_eq(
                                &curr_node,
                                &curr_node_p.borrow().get_right_child().unwrap(),
                            )
                        {
                            let temp = Rc::clone(&curr_node_p);
                            curr_node = temp;
                            self.left_rotate(Rc::clone(&curr_node));
                        }
                        let curr_node_p = curr_node.borrow().get_parent().unwrap();
                        curr_node_p.borrow_mut().update_color(Color::Black);
                        let curr_node_gp = curr_node_p.borrow().get_parent().unwrap();
                        curr_node_gp.borrow_mut().update_color(Color::Red);
                        self.right_rotate(curr_node_gp);
                    }
                };
            } else {
                let uncle_option: Option<Rc<RefCell<Node>>> =
                    curr_node_gp.borrow().get_left_child();
                match uncle_option {
                    Some(uncle) if matches!(uncle.borrow().get_color(), Color::Red) => {
                        curr_node_p.borrow_mut().update_color(Color::Black);
                        uncle.borrow_mut().update_color(Color::Black);
                        curr_node_gp.borrow_mut().update_color(Color::Red);
                    }
                    _ => {
                        if matches!(curr_node_p.borrow().get_left_child(), Some(_))
                            && Rc::ptr_eq(
                                &curr_node,
                                &curr_node_p.borrow().get_left_child().unwrap(),
                            )
                        {
                            let temp = curr_node.borrow().get_parent().unwrap();
                            curr_node = temp;
                            self.right_rotate(Rc::clone(&curr_node));
                        }
                        let curr_node_p = curr_node.borrow().get_parent().unwrap();
                        curr_node_p.borrow_mut().update_color(Color::Black);
                        let curr_node_gp = curr_node_p.borrow().get_parent().unwrap();
                        curr_node_gp.borrow_mut().update_color(Color::Red);
                        self.left_rotate(curr_node_gp);
                    }
                };
            }
        }

        if matches!(self.root, Some(_)) {
            self.root
                .as_ref()
                .unwrap()
                .borrow_mut()
                .update_color(Color::Black);
        }
    }

    fn left_rotate(&mut self, parent_node: Rc<RefCell<Node>>) {
        let right_child = parent_node.borrow().get_right_child().unwrap();
        if let Some(left) = right_child.borrow().get_left_child() {
            left.borrow_mut().parent = Some(Rc::downgrade(&parent_node));
            parent_node.borrow_mut().right = Some(left);
        } else {
            parent_node.borrow_mut().right = None;
        }

        if let Some(grand_parent) = parent_node.borrow().get_parent() {
            right_child.borrow_mut().parent = Some(Rc::downgrade(&grand_parent));

            if grand_parent.borrow().get_left_child().is_some()
                && Rc::ptr_eq(
                    &parent_node,
                    &grand_parent.borrow().get_left_child().unwrap(),
                )
            {
                grand_parent.borrow_mut().left = Some(Rc::clone(&right_child));
            } else {
                grand_parent.borrow_mut().right = Some(Rc::clone(&right_child));
            }
        } else {
            right_child.borrow_mut().parent = None;
            self.root = Some(Rc::clone(&right_child));
        }

        right_child.borrow_mut().left = Some(Rc::clone(&parent_node));
        parent_node.borrow_mut().parent = Some(Rc::downgrade(&right_child));
    }

    fn right_rotate(&mut self, parent_node: Rc<RefCell<Node>>) {
        let left_child = parent_node.borrow().get_left_child().unwrap();
        if let Some(right) = left_child.borrow().get_right_child() {
            right.borrow_mut().parent = Some(Rc::downgrade(&parent_node));
            parent_node.borrow_mut().left = Some(right);
        } else {
            parent_node.borrow_mut().left = None;
        }

        if let Some(grand_parent) = parent_node.borrow().get_parent() {
            left_child.borrow_mut().parent = Some(Rc::downgrade(&grand_parent));

            if grand_parent.borrow().get_right_child().is_some()
                && Rc::ptr_eq(
                    &parent_node,
                    &grand_parent.borrow().get_right_child().unwrap(),
                )
            {
                grand_parent.borrow_mut().right = Some(Rc::clone(&left_child));
            } else {
                grand_parent.borrow_mut().left = Some(Rc::clone(&left_child));
            }
        } else {
            left_child.borrow_mut().parent = None;
            self.root = Some(Rc::clone(&left_child));
        }

        left_child.borrow_mut().right = Some(Rc::clone(&parent_node));
        parent_node.borrow_mut().parent = Some(Rc::downgrade(&left_child));
    }

    pub fn iter(&self) -> Succesor {
        Succesor::new(self.root.as_ref())
    }
}

pub struct Succesor {
    node: Option<Rc<RefCell<Node>>>,
}

impl Succesor {
    fn new(node: Option<&Rc<RefCell<Node>>>) -> Succesor {
        if let Some(node) = node {
            Succesor {
                node: Some(Self::find_smallest(Rc::clone(node))),
            }
        } else {
            Succesor { node: None }
        }
    }

    fn find_smallest(node: Rc<RefCell<Node>>) -> Rc<RefCell<Node>> {
        let mut smallest = node;
        let mut node = smallest.borrow().get_left_child();

        while let Some(_node) = node {
            node = _node.borrow().get_left_child();
            smallest = _node;
        }
        smallest
    }
}

impl Iterator for Succesor {
    type Item = (String, i32);

    fn next(&mut self) -> Option<Self::Item> {
        let node_clone = self.node.as_ref();
        if let Some(node) = node_clone.cloned() {
            let ret = (node.borrow().key.clone(), node.borrow().value);
            if let Some(right_child) = node.borrow().get_right_child() {
                self.node = Some(Self::find_smallest(Rc::clone(&right_child)));
            } else {
                let mut child = Rc::clone(&node);
                let mut parent_op = node.borrow().get_parent();
                while let Some(parent) = parent_op.as_ref().cloned() {
                    let parent_right_child = parent.borrow().get_right_child();
                    match parent_right_child {
                        Some(right_child) if Rc::ptr_eq(&right_child, &child) => {
                            parent_op = parent.borrow().get_parent();
                            child = parent;
                        }
                        _ => {
                            break;
                        }
                    };
                }
                self.node = parent_op;
            }
            Some(ret)
        } else {
            None
        }
    }
}
