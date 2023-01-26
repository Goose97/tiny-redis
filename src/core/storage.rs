/// This module handles all the in-memory operations related to
/// storing/retrieving data
use super::Key;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq)]
pub enum StorageValue {
    String(Vec<u8>),
    Integer(isize),
    List(VecDeque<Vec<u8>>),
}

struct ValueWithExpiration(StorageValue, Option<Instant>);

struct KeyWithExpiration(Key, Instant);

impl PartialEq for KeyWithExpiration {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl Eq for KeyWithExpiration {}

impl PartialOrd for KeyWithExpiration {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // We priotize the key with smaller ttl, hence the inversion
        if self.1 < other.1 {
            Some(Ordering::Greater)
        } else if self.1 > other.1 {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

impl Ord for KeyWithExpiration {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

pub struct Storage {
    hash_map: HashMap<Vec<u8>, ValueWithExpiration>,
    key_expiration_queue: BinaryHeap<KeyWithExpiration>,
}

#[derive(Debug)]
pub enum StorageError {
    WrongOperationType,
    NotInteger,
}

pub trait ToStorageValue {
    fn to_storage_value(self) -> StorageValue;
}

impl ToStorageValue for Vec<u8> {
    fn to_storage_value(self) -> StorageValue {
        StorageValue::String(self)
    }
}

impl ToStorageValue for &str {
    fn to_storage_value(self) -> StorageValue {
        StorageValue::String(self.as_bytes().to_vec())
    }
}

impl ToStorageValue for &[u8] {
    fn to_storage_value(self) -> StorageValue {
        StorageValue::String(self.to_vec())
    }
}

impl<const N: usize> ToStorageValue for &[u8; N] {
    fn to_storage_value(self) -> StorageValue {
        StorageValue::String(self.to_vec())
    }
}

impl ToStorageValue for isize {
    fn to_storage_value(self) -> StorageValue {
        StorageValue::Integer(self)
    }
}

impl ToStorageValue for usize {
    fn to_storage_value(self) -> StorageValue {
        StorageValue::Integer(self as isize)
    }
}

impl ToStorageValue for Vec<Vec<u8>> {
    fn to_storage_value(self) -> StorageValue {
        StorageValue::List(VecDeque::from(self))
    }
}

impl ToStorageValue for Vec<String> {
    fn to_storage_value(self) -> StorageValue {
        let list = self.into_iter().map(|string| string.into_bytes()).collect();
        StorageValue::List(list)
    }
}

pub enum ListEnd {
    Front,
    Back,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            hash_map: HashMap::new(),
            key_expiration_queue: BinaryHeap::new(),
        }
    }

    pub fn get(&self, key: &Key) -> Result<Option<&StorageValue>, StorageError> {
        match self.get_raw(key) {
            None => Ok(None),
            value @ Some(StorageValue::String(_)) => Ok(value),
            value @ Some(StorageValue::Integer(_)) => Ok(value),
            _ => Err(StorageError::WrongOperationType),
        }
    }

    pub fn set<T: ToStorageValue>(&mut self, key: Key, value: T) {
        self.hash_map
            .insert(key.0, ValueWithExpiration(value.to_storage_value(), None));
    }

    // Can use negative value as decr
    pub fn incr(&mut self, key: &Key, inc: isize) -> Result<isize, StorageError> {
        match self.get_raw_mut(key) {
            None => {
                self.set(key.clone(), 1 as isize);
                Ok(1)
            }
            Some(StorageValue::Integer(integer)) => {
                *integer += inc;
                Ok(*integer)
            }
            _ => Err(StorageError::NotInteger),
        }
    }

    fn get_raw(&self, key: &Key) -> Option<&StorageValue> {
        let now = Instant::now();
        match self.hash_map.get(&key.0) {
            Some(ValueWithExpiration(_, Some(exp))) if *exp > now => None,
            Some(ValueWithExpiration(value, _)) => Some(value),
            None => None,
        }
    }

    fn get_raw_mut(&mut self, key: &Key) -> Option<&mut StorageValue> {
        let now = Instant::now();
        match self.hash_map.get_mut(&key.0) {
            Some(ValueWithExpiration(_, Some(exp))) if *exp > now => None,
            Some(ValueWithExpiration(value, _)) => Some(value),
            None => None,
        }
    }

    pub fn push(
        &mut self,
        key: Key,
        mut values: Vec<Vec<u8>>,
        list_end: ListEnd,
    ) -> Result<usize, StorageError> {
        match self.get_raw_mut(&key) {
            None => {
                let len = values.len();
                if let ListEnd::Front = list_end {
                    values.reverse();
                }

                self.set(key, values);
                Ok(len)
            }
            Some(StorageValue::List(list)) => {
                for value in values {
                    match list_end {
                        ListEnd::Front => list.push_front(value),
                        ListEnd::Back => list.push_back(value),
                    }
                }

                Ok(list.len())
            }
            _ => Err(StorageError::WrongOperationType),
        }
    }

    pub fn pop(
        &mut self,
        key: Key,
        count: usize,
        list_end: ListEnd,
    ) -> Result<Option<Vec<Vec<u8>>>, StorageError> {
        match self.get_raw_mut(&key) {
            None => Ok(None),
            Some(StorageValue::List(list)) => {
                let mut values = vec![];

                for _ in 1..=count {
                    let value = match list_end {
                        ListEnd::Front => list.pop_front(),
                        ListEnd::Back => list.pop_back(),
                    };

                    if value.is_some() {
                        values.push(value.unwrap());
                    } else {
                        break;
                    }
                }

                if values.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(values))
                }
            }
            _ => Err(StorageError::WrongOperationType),
        }
    }

    pub fn delete(&mut self, key: &Key) -> bool {
        self.hash_map.remove(&key.0).is_some()
    }

    pub fn is_exist(&self, key: &Key) -> bool {
        self.hash_map.contains_key(&key.0)
    }

    pub fn is_expire(&self, key: &Key) -> Option<bool> {
        let now = Instant::now();
        match self.hash_map.get(&key.0) {
            Some(ValueWithExpiration(_, Some(exp))) if exp <= &now => Some(true),
            Some(ValueWithExpiration(_, _)) => Some(false),
            None => None,
        }
    }

    pub fn expire(&mut self, key: &Key, ttl: u64) {
        let exp = Instant::now()
            .checked_add(Duration::from_millis(ttl))
            .unwrap();

        match self.hash_map.get_mut(&key.0) {
            Some(value) => value.1 = Some(exp.clone()),
            None => (),
        }

        self.key_expiration_queue
            .push(KeyWithExpiration(key.clone(), exp));
    }

    pub fn ttl(&self, key: &Key) -> isize {
        match self.hash_map.get(&key.0) {
            Some(ValueWithExpiration(_, Some(exp))) => {
                let ttl = exp.duration_since(Instant::now());
                ttl.as_secs() as isize
            }
            Some(ValueWithExpiration(_, None)) => -1,
            None => -2,
        }
    }

    pub fn scan_expired_keys(&mut self) -> Vec<Key> {
        let mut keys = vec![];
        let now = Instant::now();

        loop {
            match self.key_expiration_queue.peek() {
                Some(KeyWithExpiration(_, exp)) if exp > &now => break,
                Some(KeyWithExpiration(_, _)) => {
                    let KeyWithExpiration(key, _) = self.key_expiration_queue.pop().unwrap();
                    keys.push(key);
                }
                None => break,
            }
        }

        keys
    }
}

#[cfg(test)]
mod tests {
    use crate::core::storage::{ListEnd, StorageValue};

    use super::super::Key;
    use super::Storage;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn get_set() {
        let mut storage = Storage::new();
        let key = Key(b"key".to_vec());
        storage.set(key.clone(), "abc");
        assert_eq!(
            storage.get(&key).unwrap(),
            Some(&StorageValue::String(b"abc".to_vec()))
        );
    }

    #[test]
    fn incr() {
        let mut storage = Storage::new();
        let key = Key(b"key".to_vec());

        let result = storage.incr(&key, 1).unwrap();
        assert_eq!(result, 1);
        let result = storage.incr(&key, 2).unwrap();
        assert_eq!(result, 3);
        let result = storage.incr(&key, -3).unwrap();
        assert_eq!(result, 0);

        storage.set(key.clone(), "abc");
        let result = storage.incr(&key, 1);
        assert!(result.is_err());
    }

    #[test]
    fn list() {
        let mut storage = Storage::new();
        let key = Key(b"key".to_vec());

        let result: usize = storage
            .push(
                key.clone(),
                vec![b"a".to_vec(), b"b".to_vec()],
                ListEnd::Front,
            )
            .unwrap();
        assert_eq!(result, 2);

        let result: usize = storage
            .push(
                key.clone(),
                vec![b"c".to_vec(), b"d".to_vec()],
                ListEnd::Back,
            )
            .unwrap();
        assert_eq!(result, 4);

        let result = storage.pop(key.clone(), 2, ListEnd::Front).unwrap();
        assert_eq!(result, Some(vec![b"b".to_vec(), b"a".to_vec()]));
        let result = storage.pop(key.clone(), 2, ListEnd::Back).unwrap();
        assert_eq!(result, Some(vec![b"d".to_vec(), b"c".to_vec()]));
        let result = storage.pop(key.clone(), 2, ListEnd::Front).unwrap();
        assert!(result.is_none());

        storage.set(key.clone(), "abc");
        let result = storage.push(
            key.clone(),
            vec![b"c".to_vec(), b"d".to_vec()],
            ListEnd::Front,
        );
        assert!(result.is_err());
    }

    #[test]
    fn expire() {
        let mut storage = Storage::new();
        let key1 = Key(b"key1".to_vec());
        let key2 = Key(b"key2".to_vec());
        storage.expire(&key1, 2_000);
        storage.expire(&key2, 3_000);
        thread::sleep(Duration::from_secs(1));
        assert!(storage.scan_expired_keys().is_empty());

        thread::sleep(Duration::from_secs(1));
        assert_eq!(storage.scan_expired_keys(), vec![key1]);

        thread::sleep(Duration::from_secs(1));
        assert_eq!(storage.scan_expired_keys(), vec![key2]);
    }

    #[test]
    fn ttl() {
        let mut storage = Storage::new();
        let key = Key(b"key".to_vec());
        assert_eq!(storage.ttl(&key), -2);

        storage.set(key.clone(), "hello");
        assert_eq!(storage.ttl(&key), -1);

        storage.expire(&key, 2_500);
        assert_eq!(storage.ttl(&key), 2);

        thread::sleep(Duration::from_secs(2));
        let result = storage.get(&key).unwrap();
        assert!(result.is_none());
    }
}
