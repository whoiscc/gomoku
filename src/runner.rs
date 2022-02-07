use crate::collector::{Address, Collector, Owned, Shared};
use crate::interpreter::{ByteCode, Interpreter, Module, ModuleId};
use crate::objects::{Closure, Dispatch, Pending, Ready};
use crate::portal::{Portal, Task};
use crate::TaskId;
use std::sync::Arc;
use std::thread::current;

pub struct Runner {
    interp: Interpreter,
    portal: Arc<Portal>,
    collector: Arc<Collector>,
}

pub trait CollectorInterface {
    fn inspect(&self, address: Address) -> Shared;
    fn replace(&mut self, address: Address, owned: Owned) -> Owned;
    fn allocate(&mut self, handle: Owned) -> Address;
}

impl Runner {
    pub fn new(portal: Arc<Portal>, collector: Arc<Collector>) -> Self {
        let mut interp = Interpreter::new();
        interp.load_module(Module {
            id: Self::module_id(),
            symbol_table: [(Self::start_symbol(), 0)].into_iter().collect(),
            // TODO
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
        Self {
            interp,
            portal,
            collector,
        }
    }

    fn module_id() -> ModuleId {
        String::from("//task.toplevel")
    }

    fn start_symbol() -> String {
        String::from("(start)")
    }

    pub fn poll_one(&mut self) {
        let task = self.portal.fetch(current().id());
        self.collector.spawn(task.0);
        self.interp.push_variable(task.1);
        self.interp.push_call(
            Dispatch {
                module_id: Self::module_id(),
                symbol: Self::start_symbol(),
            },
            0,
        );
        while self.interp.has_step() {
            self.interp.step(&mut TaskCollector {
                collector: &*self.collector,
                task_id: task.0,
            });
        }
        let result_list = self.interp.reset();
        assert_eq!(result_list.len(), 1);
        let result = self.collector.inspect(task.0, result_list[0]);
        if result.as_ref().is::<Pending>() {
            self.portal.suspend(current().id(), task);
        } else {
            let result: &Ready = result.as_ref().downcast_ref().unwrap();
            let result = result.0;
            // TODO
            self.collector.join(task.0);
        }
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
    fn inspect(&self, address: Address) -> Shared {
        self.collector.inspect(self.task_id, address)
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
