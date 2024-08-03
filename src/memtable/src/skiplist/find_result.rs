use crate::skiplist::node::Node;

pub struct FindResult<ValueType>
where
    ValueType: Clone,
{
    pub success: bool,
    pub preds: Vec<*mut Node<ValueType>>,
    pub succs: Vec<*mut Node<ValueType>>,
}
