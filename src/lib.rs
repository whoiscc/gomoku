pub mod closure;
pub mod collector;
pub mod interpreter;
pub mod objects;
pub mod portal;
pub mod runner;

use crate::collector::EnumerateReference;
use std::any::Any;
use std::fmt::Debug;

pub trait GeneralInterface: Send + Sync + Debug + EnumerateReference + Any {
    fn as_ref(&self) -> &dyn Any;
    fn as_mut(&mut self) -> &mut dyn Any;
}
impl<T: Sized> GeneralInterface for T
where
    T: Send + Sync + Debug + collector::EnumerateReference + Any,
{
    fn as_ref(&self) -> &dyn Any {
        self
    }
    fn as_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub type TaskId = u32;
