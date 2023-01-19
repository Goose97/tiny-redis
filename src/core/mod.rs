pub mod storage;
use self::storage::Storage;

#[derive(Debug, Clone)]
pub enum Command {
    // String command
    Get(Key),
    Set(Key, Vec<u8>),
}

#[derive(Debug, PartialEq)]
pub enum CommandResponse {
    SimpleString(Vec<u8>),
    BulkString(Vec<u8>),
    Integer(isize),
    Array(Vec<CommandResponse>),
    Error(String),
    Null,
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
        match command {
            Command::Get(key) => match self.storage.get(key) {
                Some(value_string) => Ok(CommandResponse::SimpleString(value_string.0.to_owned())),
                None => Ok(CommandResponse::Null),
            },
            Command::Set(key, value) => {
                self.storage.set(key, VString(value));
                Ok(CommandResponse::SimpleString("OK".as_bytes().to_vec()))
            }
        }
    }
}

mod tests {
    use super::{Command, CommandResponse, Core, Key};

    #[test]
    fn get() {
        let mut core = Core::new();
        let key = Key(b"key".to_vec());
        let command = Command::Get(key);
        let response = core.handle_command(command).unwrap();
        assert_eq!(response, CommandResponse::Null);
    }

    #[test]
    fn set() {
        let mut core = Core::new();
        let key = Key(b"key".to_vec());
        let value = b"123".to_vec();
        let command = Command::Set(key.to_owned(), value.to_owned());
        let response = core.handle_command(command).unwrap();
        assert_response_ok(response);

        let command = Command::Get(key);
        let response = core.handle_command(command).unwrap();
        assert_eq!(response, CommandResponse::SimpleString(value));
    }

    fn assert_response_ok(response: CommandResponse) {
        let ok_response = CommandResponse::SimpleString(b"OK".to_vec());
        assert_eq!(response, ok_response);
    }
}
