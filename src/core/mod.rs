pub mod storage;
use self::storage::Storage;

#[derive(Debug, Clone)]
pub enum Command {
    // String command
    Get(Key),
    Set(Key, Vec<u8>),
    SetNx(Key, Vec<u8>),
    GetSet(Key, Vec<u8>),
}

#[derive(Debug, PartialEq)]
pub enum CommandResponse {
    SimpleString(Vec<u8>),
    BulkString(Vec<u8>),
    Integer(isize),
    Array(Vec<CommandResponse>),
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
                Command::Get(key) => match self.storage.get(&key) {
                    Some(value_string) => CommandResponse::SimpleString(value_string.0.to_owned()),
                    None => CommandResponse::Null,
                },
                Command::Set(key, value) => {
                    self.storage.set(key, VString(value));
                    CommandResponse::SimpleString("OK".as_bytes().to_vec())
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
            }
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
        assert_eq!(response, CommandResponse::SimpleString(string("123")));

        let command = Command::GetSet(key("key"), string("456"));
        let response = core.handle_command(command).unwrap();
        assert_eq!(response, CommandResponse::BulkString(string("123")));
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

    fn assert_response_ok(response: CommandResponse) {
        let ok_response = CommandResponse::SimpleString(b"OK".to_vec());
        assert_eq!(response, ok_response);
    }

    fn key(key: &str) -> Key {
        Key(key.as_bytes().to_vec())
    }

    fn string(value: &str) -> Vec<u8> {
        value.as_bytes().to_vec()
    }
}
