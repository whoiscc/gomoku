use crate::collector::Address;
use crate::TaskId;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;

#[derive(Default)]
pub struct Portal {
    peer_table: HashMap<ThreadId, Peer>,
    task_id: AtomicU32,
}

pub type Task = (TaskId, Address);

#[derive(Debug)]
struct Peer {
    poll_list: Mutex<Vec<Task>>,
    pending_set: Mutex<HashSet<Task>>,
}

impl Portal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(&self, thread_id: ThreadId, closure: Address) -> Task {
        let task_id = self.task_id.fetch_add(1, Ordering::SeqCst);
        let task = (task_id, closure);
        self.peer_table
            .get(&thread_id)
            .unwrap()
            .poll_list
            .lock()
            .unwrap()
            .push(task);
        task
    }

    pub fn suspend(&self, id: ThreadId, task: Task) {
        self.peer_table
            .get(&id)
            .unwrap()
            .pending_set
            .lock()
            .unwrap()
            .insert(task);
    }

    pub fn waker(self: &Arc<Self>, id: ThreadId, task: Task) -> Box<dyn FnOnce()> {
        let waker_self = self.clone();
        Box::new(move || {
            let peer = waker_self.peer_table.get(&id).unwrap();
            if peer.pending_set.lock().unwrap().remove(&task) {
                peer.poll_list.lock().unwrap().push(task);
            }
        })
    }

    // pub fn fetch()
}
