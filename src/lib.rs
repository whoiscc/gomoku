pub mod collector;

use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

pub trait GeneralInterface: Send + Sync + Debug + collector::IterateReference + Any {
    fn as_ref(&self) -> &dyn Any;
    fn as_mut(&mut self) -> &mut dyn Any;
}
impl<T: Sized> GeneralInterface for T
where
    T: Send + Sync + Debug + collector::IterateReference + Any,
{
    fn as_ref(&self) -> &dyn Any {
        self
    }
    fn as_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub type Handle = Arc<dyn GeneralInterface>;
pub type Address = u64;
