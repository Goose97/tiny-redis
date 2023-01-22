pub mod storage;

use self::storage::{Storage, StorageError};

#[derive(Debug, Clone)]
pub enum Command {
    // Generic commands
    Del(Vec<Key>),

    // String commands
    Get(Key),
    Set(Key, Vec<u8>),
    SetNx(Key, Vec<u8>),
    GetSet(Key, Vec<u8>),
    GetDel(Key),
    MGet(Vec<Key>),
    MSet(Vec<Key>, Vec<Vec<u8>>),
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

#[derive(Debug, Clone)]
pub struct Key(pub Vec<u8>);

// Value
pub struct VString(Vec<u8>);

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
            Command::Del(keys) => {
                let deleted_count = keys
                    .into_iter()
                    .filter(|key| self.storage.delete(key))
                    .count();
                CommandResponse::Integer(deleted_count as isize)
            }
            Command::Get(key) => self.get(&key),
            Command::Set(key, value) => {
                self.storage.set(key, VString(value));
                CommandResponse::SimpleString(b"OK")
            }
            Command::SetNx(key, value) => match self.storage.get(&key) {
                Ok(Some(_)) => CommandResponse::Integer(0),
                Ok(None) => {
                    self.storage.set(key, VString(value));
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
                    Ok(Some(VString(bytes))) => {
                        let response = CommandResponse::BulkString(bytes.to_owned());

                        // I can't find a way to extract this function to a separate closure
                        // without pissing off the borrow checker
                        match operator {
                            "GET" => {
                                self.storage.set(key, VString(value));
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
                                self.storage.set(key, VString(value));
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
                    self.storage.set(key, VString(value));
                });

                CommandResponse::SimpleString(b"OK")
            }
        }
    }

    fn get(&self, key: &Key) -> CommandResponse {
        match self.storage.get(&key) {
            Ok(Some(value_string)) => CommandResponse::SimpleString(value_string.0.as_slice()),
            Ok(None) => CommandResponse::Null,
            Err(error) => Core::translate_error(error),
        }
    }

    fn translate_error(error: StorageError) -> CommandResponse<'static> {
        match error {
            StorageError::WrongOperationType => CommandResponse::Error(String::from(
                "WRONGTYPE Operation against a key holding the wrong kind of value",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, CommandResponse, Core, Key};

    #[test]
    fn del_and_get_del() {
        let mut core = Core::new();
        let command = Command::Set(key("key1"), string("123"));
        core.handle_command(command);

        let command = Command::Del(vec![key("key1"), key("key2")]);
        let response = core.handle_command(command);
        assert_eq!(response, CommandResponse::Integer(1));

        let command = Command::Set(key("key1"), string("123"));
        core.handle_command(command);

        let command = Command::GetDel(key("key1"));
        let response = core.handle_command(command);
        assert_eq!(response, CommandResponse::BulkString(string("123")));
    }

    #[test]
    fn get_and_set() {
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

    #[test]
    fn set_nx() {
        let mut core = Core::new();
        let command = Command::SetNx(key("key"), string("123"));
        let response = core.handle_command(command);
        assert_eq!(response, CommandResponse::Integer(1));

        let command = Command::SetNx(key("key"), string("123"));
        let response = core.handle_command(command);
        assert_eq!(response, CommandResponse::Integer(0));
    }

    #[test]
    fn m_get_and_m_set() {
        let mut core = Core::new();
        let keys = vec![key("key1"), key("key2")];
        let command = Command::MGet(keys.to_owned());
        let response = core.handle_command(command);
        assert_eq!(
            response,
            CommandResponse::Array(vec![CommandResponse::Null, CommandResponse::Null])
        );

        let command = Command::MSet(
            vec![key("key1"), key("key2")],
            vec![string("123"), string("456")],
        );
        let response = core.handle_command(command);
        assert_response_ok(response);

        let command = Command::MGet(keys);
        let response = core.handle_command(command);
        assert_eq!(
            response,
            CommandResponse::Array(vec![
                CommandResponse::SimpleString(b"123"),
                CommandResponse::SimpleString(b"456")
            ])
        );
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
