use crate::collector::{Address, EnumerateReference};
use crate::interpreter::{ModuleId, OperateContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intermediate;
impl EnumerateReference for Intermediate {
    fn enumerate_reference(&self, _callback: &mut dyn FnMut(Address)) {
        panic!("intermediate placeholder escaped");
    }
}

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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct Closure {
    pub dispatch: Dispatch,
    pub capture_list: Vec<Address>,
}
impl EnumerateReference for Closure {
    fn enumerate_reference(&self, callback: &mut dyn FnMut(Address)) {
        for address in &self.capture_list {
            callback(*address);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pending;
impl LeafObject for Pending {}

#[derive(Debug, Clone)]
pub struct Ready(pub Address);
impl EnumerateReference for Ready {
    fn enumerate_reference(&self, callback: &mut dyn FnMut(Address)) {
        callback(self.0);
    }
}
impl Ready {
    // arguments: 1 variable
    // result: 1 Ready wrapping argument
    pub fn operate_new(context: &mut dyn OperateContext) {
        let ready = Self(context.get_argument(0));
        let ready = context.allocate(ready.into());
        context.push_result(ready);
    }
}
