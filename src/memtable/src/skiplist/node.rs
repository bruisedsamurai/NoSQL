﻿use std::sync::Arc;
use std::sync::atomic::AtomicPtr;

pub type KeyType = i128;

#[repr(align(2))]
#[derive(Debug)]
pub(crate) struct Node<ValueType>
where
    ValueType: Clone,
{
    pub key: KeyType,
    pub value: Option<ValueType>,
    pub top_level: usize,
    pub next: [AtomicPtr<Node<ValueType>>; 32],
}

impl<ValueType> Node<ValueType>
where
    ValueType: Clone,
{
    pub const TOP_LEVEL: usize = 31;
    pub fn new_sentinel(key: KeyType) -> *mut Node<ValueType> {
        let vec = (0..Node::<ValueType>::TOP_LEVEL + 1)
            .map(|i| AtomicPtr::new(std::ptr::null_mut()))
            .collect::<Vec<_>>();
        Box::into_raw(Box::new(Node {
            key,
            value: None,
            top_level: Node::<ValueType>::TOP_LEVEL,
            next: vec.try_into().expect("Cannot convert to array"),
        }))
    }

    pub fn new(key: KeyType, value: ValueType, height: usize) -> *mut Node<ValueType> {
        let vec = (0..Node::<ValueType>::TOP_LEVEL + 1)
            .map(|i| AtomicPtr::new(std::ptr::null_mut()))
            .collect::<Vec<_>>();
        Box::into_raw(Box::new(Node {
            key,
            value: Some(value),
            top_level: height,
            next: vec.try_into().expect("Cannot convert to array"),
        }))
    }
}
