use crate::{GeneralInterface, TaskId};
use std::collections::{HashMap, HashSet};
use std::mem::{replace, take};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, RwLock};

pub type Address = (TaskId, u32);

pub trait EnumerateReference {
    fn enumerate_reference(&self, callback: &mut dyn FnMut(Address));
}

pub struct Shared(Arc<dyn GeneralInterface>);
impl Deref for Shared {
    type Target = dyn GeneralInterface;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
#[cfg(test)]
impl From<Arc<dyn GeneralInterface>> for Shared {
    fn from(value: Arc<dyn GeneralInterface>) -> Self {
        Self(value)
    }
}

#[derive(Default)]
pub struct Collector {
    heap_table: RwLock<HashMap<TaskId, Mutex<Heap>>>,
    limbo_table: RwLock<HashMap<Address, Arc<dyn GeneralInterface>>>,
    witness_set: Mutex<HashSet<TaskId>>,
    transfer_table: RwLock<HashMap<Address, Arc<dyn GeneralInterface>>>,
}

#[derive(Default)]
struct Heap {
    storage: HashMap<Address, Arc<dyn GeneralInterface>>,
    allocate_number: u32,
}

impl Collector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(&self, id: TaskId) {
        self.heap_table
            .write()
            .unwrap()
            .insert(id, Default::default());
    }
}

pub struct Owned(Arc<dyn GeneralInterface>);
impl From<Box<dyn GeneralInterface>> for Owned {
    fn from(value: Box<dyn GeneralInterface>) -> Self {
        Self(value.into())
    }
}
impl<T: GeneralInterface> From<T> for Owned {
    fn from(value: T) -> Self {
        Self(Arc::new(value))
    }
}
#[cfg(test)]
impl Into<Arc<dyn GeneralInterface>> for Owned {
    fn into(self) -> Arc<dyn GeneralInterface> {
        self.0
    }
}
#[cfg(test)]
impl From<Arc<dyn GeneralInterface>> for Owned {
    fn from(value: Arc<dyn GeneralInterface>) -> Self {
        Self(value)
    }
}
impl Deref for Owned {
    type Target = dyn GeneralInterface;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
impl DerefMut for Owned {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::get_mut(&mut self.0).unwrap()
    }
}

impl Collector {
    pub fn allocate(&self, id: TaskId, owned: Owned) -> Address {
        let heap_table = self.heap_table.read().unwrap();
        let mut heap = heap_table.get(&id).unwrap().lock().unwrap();
        heap.allocate_number += 1;
        let address = (id, heap.allocate_number);
        heap.storage.insert(address, owned.0);
        address
    }

    pub fn inspect(&self, id: TaskId, address: Address) -> Shared {
        let heap_table = self.heap_table.read().unwrap();
        let mut heap = heap_table.get(&id).unwrap().lock().unwrap();
        Shared(self.inspect_internal(address, &*heap_table, &mut *heap))
    }

    fn inspect_internal(
        &self,
        address: Address,
        heap_table: &HashMap<TaskId, Mutex<Heap>>,
        heap: &mut Heap,
    ) -> Arc<dyn GeneralInterface> {
        if let Some(shared) = heap.storage.get(&address) {
            shared.clone()
        } else {
            let shared = (|| {
                let limbo_table = self.limbo_table.read().unwrap();
                let transfer_table = self.transfer_table.read().unwrap();
                if let Some(remote_heap) = heap_table.get(&address.0) {
                    if let Some(shared) = remote_heap.lock().unwrap().storage.get(&address) {
                        return shared.clone();
                    }
                }
                if let Some(shared) = transfer_table.get(&address) {
                    return shared.clone();
                }
                limbo_table.get(&address).unwrap().clone()
            })();
            heap.storage.insert(address, shared.clone());
            shared
        }
    }

    pub fn replace_owned(&self, address: Address, owned: Owned) -> Owned {
        let heap_table = self.heap_table.read().unwrap();
        let mut heap = heap_table.get(&address.0).unwrap().lock().unwrap();
        let replaced = heap.storage.insert(address, owned.0).unwrap();
        assert_eq!(Arc::strong_count(&replaced), 1);
        Owned(replaced)
    }

    pub fn copy_collect(&self, id: TaskId, root_list: &[Address]) {
        let mut gray_list = root_list.to_vec();
        let mut storage = HashMap::new();

        let heap_table = self.heap_table.read().unwrap();
        let mut heap = heap_table.get(&id).unwrap().lock().unwrap();
        while let Some(address) = gray_list.pop() {
            let shared = self.inspect_internal(address, &*heap_table, &mut *heap);
            storage.insert(address, shared.clone());
            shared.enumerate_reference(&mut |address| {
                if !storage.contains_key(&address) {
                    gray_list.push(address);
                }
            });
        }
        let collected = replace(&mut heap.storage, storage);
        self.transfer_table.write().unwrap().extend(collected);
        self.witness_set.lock().unwrap().remove(&id);
    }

    pub fn join(&self, id: TaskId) {
        self.copy_collect(id, &[]);
        self.heap_table.write().unwrap().remove(&id);
    }

    pub fn epoch_change<F: FnOnce() -> HashSet<TaskId>>(&self, witness_set: F) {
        let mut previous_witness_set = self.witness_set.lock().unwrap();
        if !previous_witness_set.is_empty() {
            return;
        }
        let mut limbo_table = self.limbo_table.write().unwrap();
        let mut transfer_table = self.transfer_table.write().unwrap();
        let previous_transfer_table = take(&mut *transfer_table);
        let _ = replace(&mut *limbo_table, previous_transfer_table);
        let _ = replace(&mut *previous_witness_set, witness_set());
    }
}
