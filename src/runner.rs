use crate::collector::Owned;
use crate::collector::{Address, Collector};
use crate::interpreter::{ByteCode, Interpreter, Module, ModuleId};
use crate::objects::{Closure, Dispatch, False, True};
use crate::portal::{Portal, Task};
use crate::GeneralInterface;
use crate::TaskId;
use std::ops::Deref;
use std::sync::Arc;
use std::thread::current;

pub struct Runner {
    interp: Interpreter,
    portal: Arc<Portal>,
    collector: Arc<Collector>,
}

pub type Inspect = Box<dyn Deref<Target = dyn GeneralInterface>>;
pub trait CollectorInterface {
    fn inspect(&self, address: Address) -> Inspect;
    fn replace(&mut self, address: Address, owned: Owned) -> Owned;
    fn allocate(&mut self, handle: Owned) -> Address;
}

impl Runner {
    pub fn new(portal: Arc<Portal>, collector: Arc<Collector>) -> Self {
        Self {
            interp: Interpreter::new(),
            portal,
            collector,
        }
    }

    fn module_id() -> ModuleId {
        String::from("//async")
    }

    fn start_symbol() -> String {
        String::from("(start)")
    }

    pub fn prepare_task(&mut self) {
        self.interp.load_module(Module {
            id: Self::module_id(),
            symbol_table: [(Self::start_symbol(), 0)].into_iter().collect(),
            program: vec![
                ByteCode::AssertFloating(1),
                ByteCode::Operate(1, Box::new(Closure::operate_apply)),
                ByteCode::Call(1),
                ByteCode::PackFloating(1),
                // ready flag, capture pack, extracted result, updated task
                // ByteCode::Operate(3, Box::new(Closure::operate_poll)),
                ByteCode::Copy(4),
                ByteCode::Copy(4),
                ByteCode::Return(3), // (order in result list) ready flag, updated task, extracted result
            ],
        });
    }
}

struct TaskCollector<'a> {
    collector: &'a Collector,
    task_id: TaskId,
}
impl<'a> CollectorInterface for TaskCollector<'a> {
    fn allocate(&mut self, owned: Owned) -> Address {
        self.collector.allocate(self.task_id, owned)
    }
    fn inspect(&self, address: Address) -> Inspect {
        Box::new(self.collector.inspect(self.task_id, address))
    }
    fn replace(&mut self, address: Address, owned: Owned) -> Owned {
        self.collector.replace_owned(address, owned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_task() {
        // let mut runner = Runner::new();
        // assert!(runner.prepare_task().is_none());
    }
}
