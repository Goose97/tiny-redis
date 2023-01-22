/// This module handles all the in-memory operations related to
/// storing/retrieving data
use super::{Key, VString};
use std::collections::HashMap;

enum StorageValue {
    String(VString),
}

pub struct Storage {
    hash_map: HashMap<Vec<u8>, StorageValue>,
}

pub enum StorageError {
    WrongOperationType,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            hash_map: HashMap::new(),
        }
    }

    pub fn get(&self, key: &Key) -> Result<Option<&VString>, StorageError> {
        match self.hash_map.get(&key.0) {
            Some(StorageValue::String(value)) => Ok(Some(value)),
            Some(_other_type) => Err(StorageError::WrongOperationType),
            None => Ok(None),
        }
    }

    pub fn set(&mut self, key: Key, value: VString) {
        self.hash_map.insert(key.0, StorageValue::String(value));
    }
}
