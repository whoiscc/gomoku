use crate::{GeneralInterface, Handle};
use std::collections::HashMap;
use std::mem::replace;
use std::sync::Arc;
use std::thread::{current, ThreadId};

pub type Address = (ThreadId, u64);

pub trait EnumerateReference {
    fn enumerate_reference(&self, callback: &mut dyn FnMut(Address));
}

#[derive(Debug)]
pub struct Collector {
    storage: HashMap<Address, Handle>,
    allocate_number: u64,
}

impl Collector {
    pub fn new() -> Self {
        Self {
            storage: HashMap::new(),
            allocate_number: 0,
        }
    }

    pub fn allocate(&mut self, handle: Handle) -> Address {
        self.allocate_number += 1;
        let address = (current().id(), self.allocate_number);
        self.storage.insert(address, handle);
        address
    }

    pub fn copy_collect(&mut self, address_list: &[Address]) -> HashMap<Address, Handle> {
        let mut gray_stack = address_list.to_vec();
        let mut storage = HashMap::new();
        while let Some(address) = gray_stack.pop() {
            let handle = self.storage.get(&address).unwrap();
            storage.insert(address, handle.clone());
            handle.enumerate_reference(&mut |address| {
                if !storage.contains_key(&address) {
                    gray_stack.push(address);
                }
            });
        }
        replace(&mut self.storage, storage)
    }

    pub fn inspect(&self, address: Address) -> &dyn GeneralInterface {
        &**self.storage.get(&address).unwrap()
    }

    pub fn inspect_mut(&mut self, address: Address) -> &mut dyn GeneralInterface {
        Arc::get_mut(self.storage.get_mut(&address).unwrap()).unwrap()
    }

    pub fn count(&self) -> usize {
        self.storage.len()
    }
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct GeneralNode(u64, Vec<Address>);
    impl EnumerateReference for GeneralNode {
        fn enumerate_reference(&self, callback: &mut dyn FnMut(Address)) {
            for address in &self.1 {
                callback(*address);
            }
        }
    }

    #[test]
    fn collect_all() {
        let mut c = Collector::new();
        for i in 0..10 {
            c.allocate(Arc::new(GeneralNode(i, Vec::new())));
        }
        assert_eq!(c.count(), 10);
        c.copy_collect(&[]);
        assert_eq!(c.count(), 0);
    }

    #[test]
    fn collect_orphan() {
        let mut c = Collector::new();
        let mut root = GeneralNode(0, Vec::new());
        let mut side_list = Vec::new();
        for i in 0..10 {
            let handle = Arc::new(GeneralNode(i, Vec::new()));
            side_list.push(Arc::downgrade(&handle));
            let address = c.allocate(handle);
            if i < 7 {
                root.1.push(address);
            }
        }
        let root_list = [c.allocate(Arc::new(root))];
        c.copy_collect(&root_list);
        assert_eq!(c.count(), 7 + 1);
        for weak in side_list {
            if let Some(handle) = weak.upgrade() {
                assert!(handle.0 < 7);
            }
        }
    }

    #[test]
    fn collect_cyclic_orphan() {
        let mut c = Collector::new();
        let address1 = c.allocate(Arc::new(GeneralNode(0, Vec::new())));
        let address2 = c.allocate(Arc::new(GeneralNode(0, Vec::new())));
        c.inspect_mut(address1)
            .as_mut()
            .downcast_mut::<GeneralNode>()
            .unwrap()
            .1
            .push(address2);
        c.inspect_mut(address2)
            .as_mut()
            .downcast_mut::<GeneralNode>()
            .unwrap()
            .1
            .push(address1);
        c.copy_collect(&[]);
        assert_eq!(c.count(), 0);
    }
}
