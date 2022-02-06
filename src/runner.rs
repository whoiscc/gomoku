use crate::collector::Address;
use crate::interpreter::{ByteCode, Interpreter, Module, ModuleId};
use crate::objects::{Closure, Dispatch, False, True};
use std::thread::current;

pub struct Runner {
    interp: Interpreter,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            interp: Interpreter::new(),
            // TODO
        }
    }

    pub fn garbage_collect(&mut self) {
        todo!()
    }

    fn module_id() -> ModuleId {
        String::from("//async")
    }

    fn start_symbol() -> String {
        String::from("(start)")
    }

    pub fn prepare_task(&mut self) -> Option<Address> {
        let task_address = (current().id(), 0); // TODO

        self.interp.load_module(Module {
            id: Self::module_id(),
            symbol_table: [(Self::start_symbol(), 0)].into_iter().collect(),
            program: vec![
                ByteCode::AssertFloating(1),
                ByteCode::Operate(1, Box::new(Closure::operate_apply)),
                ByteCode::Call(1),
                ByteCode::PackFloating(1),
                // ready flag, capture pack, extracted result, updated task
                ByteCode::Operate(3, Box::new(Closure::operate_poll)),
                ByteCode::Copy(4),
                ByteCode::Copy(4),
                ByteCode::Return(3), // (order in result list) ready flag, updated task, extracted result
            ],
        });
        self.interp.push_variable(task_address);
        self.interp.push_call(
            Dispatch {
                module_id: Self::module_id(),
                symbol: Self::start_symbol(),
            },
            0,
        );
        Some(task_address)
    }

    pub fn poll_task(&mut self) -> Option<Address> {
        while self.interp.has_step() {
            self.interp.step();
        }
        let result_list = self.interp.reset();
        let ready = self.interp.collector.inspect(result_list[0]).as_ref();
        if ready.is::<False>() {
            todo!()
        }
        if ready.is::<True>() {
            return Some(result_list[2]);
        }
        unreachable!()
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
