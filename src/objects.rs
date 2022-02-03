use crate::collector::IterateReference;
use crate::Address;

pub trait LeafObject {}
impl<T: LeafObject> IterateReference for T {
    fn iterate_reference(&self, _c: &mut dyn FnMut(Address)) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct True;
impl LeafObject for True {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct False;
impl LeafObject for False {}
