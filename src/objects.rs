use crate::collector::EnumerateReference;
use crate::interpreter::ModuleId;
use crate::{Address, Handle};

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

#[derive(Debug, Clone)]
pub struct Dispatch {
    pub module_id: ModuleId,
    pub symbol: String,
    // debug print
}
impl LeafObject for Dispatch {}

/// The literal of closure, present in byte code
#[derive(Debug)]
pub struct ClosureMeta {
    pub dispatch: Dispatch,
    pub stage_list: Vec<u8>, // local variable count on every async stage
    pub n_capture: u8,
    pub n_argument: u8,
}
impl LeafObject for ClosureMeta {}

/// The runtime closure object, before first invoking
#[derive(Debug)]
pub struct Closure {
    pub dispatch: Dispatch,
    pub stage_list: Vec<u8>,
    pub capture_list: Vec<Address>,
    pub n_argument: u8,
}
impl EnumerateReference for Closure {
    fn enumerate_reference(&self, callback: &mut dyn FnMut(Address)) {
        for address in &self.capture_list {
            callback(*address);
        }
    }
}

/// The runtime object for a coroutine, turned from a closure object after invoking
#[derive(Debug)]
pub struct AsyncState {
    pub dispatch: Dispatch,
    pub stage: u8,
    pub variable_list: Vec<Handle>,
}
impl LeafObject for AsyncState {}
