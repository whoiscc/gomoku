use crate::collector::EnumerateReference;
use crate::interpreter::ModuleId;
use crate::Address;

pub trait LeafObject {}
impl<T: LeafObject> EnumerateReference for T {
    fn enumerate_reference(&self, _c: &mut dyn FnMut(Address)) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct True;
impl LeafObject for True {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct False;
impl LeafObject for False {}

#[derive(Debug)]
pub struct List(pub Vec<Address>);
impl EnumerateReference for List {
    fn enumerate_reference(&self, callback: &mut dyn FnMut(Address)) {
        for element in &self.0 {
            callback(*element);
        }
    }
}

#[derive(Debug)]
pub struct Dispatch {
    pub module_id: ModuleId,
    pub symbol: String,
    // debug print
}
impl LeafObject for Dispatch {}

#[derive(Debug)]
pub struct ClosureMeta {
    pub dispatch: Dispatch,
    pub n_capture: u8,
    pub n_argument: u8,
}
