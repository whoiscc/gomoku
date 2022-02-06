use crate::interpreter::Interpreter;
use std::collections::HashMap;
use std::sync::Mutex;
use std::thread::ThreadId;

pub struct Portal {
    peer_table: HashMap<ThreadId, Peer>,
}

struct Peer {
    collect_mutex: Mutex<()>,
}

impl Portal {
    pub fn new() -> Self {
        // todo
        Self {
            peer_table: HashMap::new(),
        }
    }

    fn lock_collect(&self, id: ThreadId) -> impl Drop + '_ {
        self.peer_table
            .get(&id)
            .unwrap()
            .collect_mutex
            .lock()
            .unwrap()
    }

    pub fn garbage_collect(&self, id: ThreadId, interp: &mut Interpreter) {
        let guard = self.lock_collect(id);
        interp.garbage_collect(&[]); // TODO
        drop(guard);
    }
}
