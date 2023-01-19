/// This module handles all the in-memory operations related to
/// storing/retrieving data
use super::{Key, VString};
use std::collections::HashMap;

pub struct Storage {
    hash_map: HashMap<Vec<u8>, VString>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            hash_map: HashMap::new(),
        }
    }

    pub fn get(&self, key: Key) -> Option<&VString> {
        self.hash_map.get(&key.0)
    }

    pub fn set(&mut self, key: Key, value: VString) {
        self.hash_map.insert(key.0, value);
    }
}
