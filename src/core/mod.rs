pub mod storage;

use self::storage::{ListEnd, Storage, StorageError, StorageValue};

#[derive(Debug, Clone)]
pub enum Command {
    // Generic commands
    Del(Vec<Key>),
    Expire(Key, usize),
    Ttl(Key),
    Exists(Vec<Key>),
    Flush,

    // String commands
    Get(Key),
    Set(Key, Vec<u8>),
    SetNx(Key, Vec<u8>),
    GetSet(Key, Vec<u8>),
    GetDel(Key),
    MGet(Vec<Key>),
    MSet(Vec<Key>, Vec<Vec<u8>>),
    Incr(Key),
    Decr(Key),
    IncrBy(Key, Vec<u8>),
    DecrBy(Key, Vec<u8>),

    // List commands
    LPush(Key, Vec<Vec<u8>>),
    RPush(Key, Vec<Vec<u8>>),
    LPop(Key, usize),
    RPop(Key, usize),

    // Internal commands
    ExpIntervalCheck,
}

#[derive(Debug, PartialEq)]
pub enum CommandResponse<'a> {
    SimpleString(&'a [u8]),
    BulkString(Vec<u8>),
    Integer(isize),
    Array(Vec<CommandResponse<'a>>),
    Error(String),
    Null,
    NullArray,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Key(pub Vec<u8>);

pub struct Core {
    storage: Storage,
}

impl Core {
    pub fn new() -> Self {
        Self {
            storage: Storage::new(),
        }
    }

    pub fn handle_command(&mut self, command: Command) -> CommandResponse {
        match command {
            Command::ExpIntervalCheck => {
                for key in self.storage.scan_expired_keys() {
                    if let Some(true) = self.storage.is_expire(&key) {
                        self.storage.delete(&key);
                    }
                }

                CommandResponse::Null
            }

            Command::Del(keys) => {
                let deleted_count = keys
                    .into_iter()
                    .filter(|key| self.storage.delete(key))
                    .count();
                CommandResponse::Integer(deleted_count as isize)
            }

            Command::Expire(key, ttl) => {
                if self.storage.is_exist(&key) {
                    let milliseconds: u64 = u64::try_from(ttl * 1000).unwrap();
                    self.storage.expire(&key, milliseconds);
                    CommandResponse::Integer(1)
                } else {
                    CommandResponse::Integer(0)
                }
            }

            Command::Ttl(key) => {
                let ttl = self.storage.ttl(&key);
                CommandResponse::Integer(ttl)
            }

            Command::Exists(keys) => {
                let count = keys
                    .into_iter()
                    .filter(|key| self.storage.is_exist(key))
                    .count();
                CommandResponse::Integer(count as isize)
            }

            Command::Flush => {
                self.storage = Storage::new();
                CommandResponse::SimpleString(b"OK")
            }

            Command::Get(key) => self.get(&key),

            Command::Set(key, value) => {
                self.storage.set(key, value);
                CommandResponse::SimpleString(b"OK")
            }

            Command::SetNx(key, value) => match self.storage.get(&key) {
                Ok(Some(_)) => CommandResponse::Integer(0),
                Ok(None) => {
                    self.storage.set(key, value);
                    CommandResponse::Integer(1)
                }
                Err(error) => Core::translate_error(error),
            },

            command @ (Command::GetSet(_, _) | Command::GetDel(_)) => {
                let operator = if matches!(command, Command::GetSet(_, _)) {
                    "GET"
                } else if matches!(command, Command::GetDel(_)) {
                    "DEL"
                } else {
                    unreachable!();
                };

                let (key, value) = match command {
                    Command::GetSet(key, value) => (key, value),
                    // Leave it as vec so compiler doesn't complain about ununiformed types
                    Command::GetDel(key) => (key, vec![]),
                    _ => unreachable!(),
                };

                match self.storage.get(&key) {
                    Ok(Some(old_value)) => {
                        let response = match old_value {
                            StorageValue::String(bytes) => {
                                CommandResponse::BulkString(bytes.to_owned())
                            }
                            StorageValue::Integer(integer) => {
                                CommandResponse::BulkString(integer.to_string().into_bytes())
                            }
                            StorageValue::List(_) => return CommandResponse::Error(String::from(
                                "WRONGTYPE Operation against a key holding the wrong kind of value",
                            )),
                        };

                        // I can't find a way to extract this function to a separate closure
                        // without pissing off the borrow checker
                        match operator {
                            "GET" => {
                                self.storage.set(key, value);
                            }
                            "DEL" => {
                                self.storage.delete(&key);
                            }
                            _ => unreachable!(),
                        };

                        response
                    }
                    Ok(None) => {
                        match operator {
                            "GET" => {
                                self.storage.set(key, value);
                            }
                            "DEL" => {
                                self.storage.delete(&key);
                            }
                            _ => unreachable!(),
                        };

                        CommandResponse::Null
                    }
                    Err(error) => Core::translate_error(error),
                }
            }

            Command::MGet(keys) => {
                let values = keys
                    .iter()
                    .map(|key| self.get(&key))
                    .collect::<Vec<CommandResponse>>();

                CommandResponse::Array(values)
            }

            Command::MSet(keys, mut values) => {
                keys.into_iter().for_each(|key| {
                    let value = values.remove(0);
                    self.storage.set(key, value);
                });

                CommandResponse::SimpleString(b"OK")
            }

            command @ (Command::Incr(_) | Command::Decr(_)) => match command {
                Command::Incr(key) => match self.storage.incr(&key, 1) {
                    Ok(new_value) => CommandResponse::Integer(new_value),
                    Err(error) => Core::translate_error(error),
                },
                Command::Decr(key) => match self.storage.incr(&key, -1) {
                    Ok(new_value) => CommandResponse::Integer(new_value),
                    Err(error) => Core::translate_error(error),
                },
                _ => unreachable!(),
            },

            command @ (Command::IncrBy(_, _) | Command::DecrBy(_, _)) => {
                let negative = if matches!(command, Command::IncrBy(_, _)) {
                    false
                } else if matches!(command, Command::DecrBy(_, _)) {
                    true
                } else {
                    unreachable!()
                };

                let (key, value) = match command {
                    Command::IncrBy(key, value) => (key, value),
                    Command::DecrBy(key, value) => (key, value),
                    _ => unreachable!(),
                };

                match String::from_utf8(value)
                    .ok()
                    .and_then(|string| string.parse::<isize>().ok())
                {
                    Some(integer) => {
                        let value = if negative { -integer } else { integer };

                        match self.storage.incr(&key, value) {
                            Ok(new_value) => CommandResponse::Integer(new_value),
                            Err(error) => Core::translate_error(error),
                        }
                    }
                    None => CommandResponse::Error(String::from(
                        "ERR value is not an integer or out of range",
                    )),
                }
            }

            Command::LPush(key, values) => match self.storage.push(key, values, ListEnd::Front) {
                Ok(size) => CommandResponse::Integer(size as isize),
                Err(error) => Core::translate_error(error),
            },

            Command::RPush(key, values) => match self.storage.push(key, values, ListEnd::Back) {
                Ok(size) => CommandResponse::Integer(size as isize),
                Err(error) => Core::translate_error(error),
            },

            Command::LPop(key, count) => match self.storage.pop(key, count, ListEnd::Front) {
                Ok(None) => CommandResponse::Null,
                Ok(Some(values)) => {
                    let items = values
                        .into_iter()
                        .map(|item| CommandResponse::BulkString(item))
                        .collect::<Vec<_>>();
                    CommandResponse::Array(items)
                }
                Err(error) => Core::translate_error(error),
            },

            Command::RPop(key, count) => match self.storage.pop(key, count, ListEnd::Back) {
                Ok(None) => CommandResponse::Null,
                Ok(Some(values)) => {
                    let items = values
                        .into_iter()
                        .map(|item| CommandResponse::BulkString(item))
                        .collect::<Vec<_>>();
                    CommandResponse::Array(items)
                }
                Err(error) => Core::translate_error(error),
            },
        }
    }

    fn get(&self, key: &Key) -> CommandResponse {
        match self.storage.get(&key) {
            Ok(Some(StorageValue::String(value_string))) => {
                CommandResponse::SimpleString(value_string)
            }
            Ok(Some(StorageValue::Integer(integer))) => CommandResponse::Integer(integer.clone()),
            Ok(Some(StorageValue::List(_))) => CommandResponse::Error(String::from(
                "WRONGTYPE Operation against a key holding the wrong kind of value",
            )),
            Ok(None) => CommandResponse::Null,
            Err(error) => Core::translate_error(error),
        }
    }

    fn translate_error(error: StorageError) -> CommandResponse<'static> {
        match error {
            StorageError::WrongOperationType => CommandResponse::Error(String::from(
                "WRONGTYPE Operation against a key holding the wrong kind of value",
            )),
            StorageError::NotInteger => {
                CommandResponse::Error(String::from("ERR value is not an integer or out of range"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, CommandResponse, Core, Key};

    #[test]
    fn it_works() {
        let mut core = Core::new();
        let command = Command::Get(key("key"));
        let response = core.handle_command(command);
        assert_eq!(response, CommandResponse::Null);

        let command = Command::Set(key("key"), string("123"));
        let response = core.handle_command(command);
        assert_response_ok(response);

        let command = Command::Get(key("key"));
        let response = core.handle_command(command);
        assert_eq!(response, CommandResponse::SimpleString(b"123"));

        let command = Command::GetSet(key("key"), string("456"));
        let response = core.handle_command(command);
        assert_eq!(response, CommandResponse::BulkString(b"123".to_vec()));
    }

    fn assert_response_ok(response: CommandResponse) {
        let ok_response = CommandResponse::SimpleString(b"OK");
        assert_eq!(response, ok_response);
    }

    fn key(key: &str) -> Key {
        Key(key.as_bytes().to_vec())
    }

    fn string(value: &str) -> Vec<u8> {
        value.as_bytes().to_vec()
    }
}
