use crate::collector::Address;
use crate::interpreter::{ByteCode, Interpreter, Module, ModuleId};
use crate::objects::{Closure, Dispatch, False, True};
use crate::WeakHandle;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::{current, ThreadId};

pub struct Runner {
    interp: Interpreter,
    peer_table: Arc<HashMap<ThreadId, Peer>>,
}

struct Task {
    address: Address,
    export_table: HashMap<Address, WeakHandle>,
}

pub struct Peer {
    task_list: Mutex<Vec<Task>>,
    // something more?
}

impl Runner {
    pub fn new() -> Self {
        Self {
            interp: Interpreter::new(),
            peer_table: Arc::new(HashMap::default()),
        }
    }

    pub fn set_peer_table(&mut self, peer_table: Arc<HashMap<ThreadId, Peer>>) {
        self.peer_table = peer_table;
    }

    pub fn garbage_collect(&mut self) {
        let task_list = self
            .peer_table
            .get(&current().id())
            .unwrap()
            .task_list
            .lock()
            .unwrap();
        self.interp.garbage_collect(
            &task_list
                .iter()
                .map(|task| task.address)
                .collect::<Vec<_>>(),
        );
        drop(task_list);
    }

    fn module_id() -> ModuleId {
        String::from("//async")
    }

    fn start_symbol() -> String {
        String::from("(start)")
    }

    pub fn prepare_task(&mut self) -> Option<Address> {
        let task_address = (|| {
            if let Some(task) = self
                .peer_table
                .get(&current().id())
                .unwrap()
                .task_list
                .lock()
                .unwrap()
                .pop()
            {
                Some(task.address)
            } else {
                for (thread_id, peer) in self.peer_table.iter() {
                    if *thread_id == current().id() {
                        continue;
                    }
                    let mut task_list = peer.task_list.lock().unwrap();
                    if let Some(task) = task_list.pop() {
                        for (address, weak_handle) in task.export_table.into_iter() {
                            self.interp.collector.import(address, weak_handle);
                        }
                        drop(task_list);
                        return Some(task.address);
                    }
                }
                None
            }
        })();
        if task_address.is_none() {
            return task_address;
        }
        let task_address = task_address.unwrap();

        self.interp.load_module(Module {
            id: Self::module_id(),
            symbol_table: [(Self::start_symbol(), 0)].into_iter().collect(),
            program: vec![
                ByteCode::AssertFloating(1),
                ByteCode::Operate(1, Box::new(Closure::operate_apply)),
                ByteCode::Call(1),
                ByteCode::PackFloating(1),
                ByteCode::Operate(3, Box::new(Closure::operate_poll)),
                ByteCode::Copy(3),
                ByteCode::Return(2),
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
            return None;
        }
        if ready.is::<True>() {
            return Some(result_list[1]);
        }
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_task() {
        let mut runner = Runner::new();
        runner.set_peer_table(Arc::new(
            [(
                current().id(),
                Peer {
                    task_list: Mutex::new(Vec::new()),
                },
            )]
            .into_iter()
            .collect(),
        ));
        assert!(runner.prepare_task().is_none());
    }
}
