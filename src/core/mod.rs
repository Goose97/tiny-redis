pub mod storage;

use self::storage::Storage;

#[derive(Debug, Clone)]
pub enum Command {
    // String command
    Get(Key),
    Set(Key, Vec<u8>),
    SetNx(Key, Vec<u8>),
    GetSet(Key, Vec<u8>),
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

#[derive(Debug)]
pub enum CommandError {}

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

    pub fn handle_command(&mut self, command: Command) -> Result<CommandResponse, CommandError> {
        try {
            match command {
                Command::Get(key) => self.get(&key),
                Command::Set(key, value) => {
                    self.storage.set(key, VString(value));
                    CommandResponse::SimpleString(b"OK")
                }
                Command::SetNx(key, value) => match self.storage.get(&key) {
                    Some(_) => CommandResponse::Integer(0),
                    None => {
                        self.storage.set(key, VString(value));
                        CommandResponse::Integer(1)
                    }
                },
                Command::GetSet(key, value) => match self.storage.set(key, VString(value)) {
                    Some(VString(bytes)) => CommandResponse::BulkString(bytes),
                    None => CommandResponse::Null,
                },
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
    }

    fn get(&self, key: &Key) -> CommandResponse {
        match self.storage.get(&key) {
            Some(value_string) => CommandResponse::SimpleString(value_string.0.as_slice()),
            None => CommandResponse::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, CommandResponse, Core, Key};

    #[test]
    fn get_and_set() {
        let mut core = Core::new();
        let command = Command::Get(key("key"));
        let response = core.handle_command(command).unwrap();
        assert_eq!(response, CommandResponse::Null);

        let command = Command::Set(key("key"), string("123"));
        let response = core.handle_command(command).unwrap();
        assert_response_ok(response);

        let command = Command::Get(key("key"));
        let response = core.handle_command(command).unwrap();
        assert_eq!(response, CommandResponse::SimpleString(b"123"));

        let command = Command::GetSet(key("key"), string("456"));
        let response = core.handle_command(command).unwrap();
        assert_eq!(response, CommandResponse::BulkString(b"123".to_vec()));
    }

    #[test]
    fn set_nx() {
        let mut core = Core::new();
        let command = Command::SetNx(key("key"), string("123"));
        let response = core.handle_command(command).unwrap();
        assert_eq!(response, CommandResponse::Integer(1));

        let command = Command::SetNx(key("key"), string("123"));
        let response = core.handle_command(command).unwrap();
        assert_eq!(response, CommandResponse::Integer(0));
    }

    #[test]
    fn m_get_and_m_set() {
        let mut core = Core::new();
        let keys = vec![key("key1"), key("key2")];
        let command = Command::MGet(keys.to_owned());
        let response = core.handle_command(command).unwrap();
        assert_eq!(
            response,
            CommandResponse::Array(vec![CommandResponse::Null, CommandResponse::Null])
        );

        let command = Command::MSet(
            vec![key("key1"), key("key2")],
            vec![string("123"), string("456")],
        );
        let response = core.handle_command(command).unwrap();
        assert_response_ok(response);

        let command = Command::MGet(keys);
        let response = core.handle_command(command).unwrap();
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
