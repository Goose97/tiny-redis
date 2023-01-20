/// Handle inbound requests a.k.a commands
/// The main interface is CommandIter struct which is an iterator
/// over commands
use std::{
    borrow::Borrow,
    io,
    io::{BufRead, BufReader, Read},
    str,
};

use crate::core::{Command, Key};

pub struct CommandIter<T: Read>(pub TokenIter<T>);

#[derive(Debug)]
pub enum Error {
    KeyNotFound,
    UnexpectedToken { expect: Token, found: Option<Token> },
    MissingArguments(usize),
    MissingCrlf,
    IoError(io::Error),
}

impl<T: Read> CommandIter<T> {
    pub fn new(stream: T) -> Self {
        Self(TokenIter(BufReader::new(stream)))
    }

    fn parse(&mut self) -> Result<Command, Error> {
        let mut token_iter = &mut self.0;
        let command_size = command_size(&mut token_iter)?;
        let command = command(&mut token_iter)?;
        let mut arguments = arguments(&mut token_iter, command_size - 1)?;

        try {
            match command.borrow() {
                "GET" => {
                    let key = expect_key(&mut arguments)?;
                    Command::Get(key)
                }

                "SET" => {
                    let key = expect_key(&mut arguments)?;
                    let value = expect_binary(&mut arguments)?;
                    Command::Set(key, value)
                }

                "SETNX" => {
                    let key = expect_key(&mut arguments)?;
                    let value = expect_binary(&mut arguments)?;
                    Command::SetNx(key, value)
                }

                "GETSET" => {
                    let key = expect_key(&mut arguments)?;
                    let value = expect_binary(&mut arguments)?;
                    Command::GetSet(key, value)
                }

                "MGET" => {
                    let mut keys = vec![];
                    while !arguments.is_empty() {
                        let key = expect_key(&mut arguments)?;
                        keys.push(key);
                    }
                    Command::MGet(keys)
                }

                "MSET" => {
                    let mut keys = vec![];
                    let mut values = vec![];
                    let mut index = 0;

                    while !arguments.is_empty() {
                        if index % 2 == 0 {
                            let key = expect_key(&mut arguments)?;
                            keys.push(key);
                        } else {
                            let value = expect_binary(&mut arguments)?;
                            values.push(value);
                        }

                        index += 1;
                    }

                    if keys.len() != values.len() {
                        return Err(Error::MissingArguments(1));
                    } else {
                        Command::MSet(keys, values)
                    }
                }

                _ => unimplemented!(),
            }
        }
    }
}

impl<T: Read> Iterator for CommandIter<T> {
    type Item = Command;

    fn next(&mut self) -> Option<Self::Item> {
        self.parse().ok()
    }
}

fn command_size<T: Read>(token_iter: &mut TokenIter<T>) -> Result<usize, Error> {
    let token = token_iter.next();

    if let Some(Token::Array(size)) = token {
        Ok(size)
    } else {
        Err(Error::UnexpectedToken {
            expect: Token::Array(0),
            found: None,
        })
    }
}

fn command<T: Read>(token_iter: &mut TokenIter<T>) -> Result<String, Error> {
    let token = token_iter.next();
    if let Some(Token::String(command)) = token {
        Ok(bytes_to_string(command))
    } else {
        Err(Error::UnexpectedToken {
            expect: Token::String(vec![]),
            found: None,
        })
    }
}

fn arguments<T: Read>(token_iter: &mut TokenIter<T>, num: usize) -> Result<Vec<Token>, Error> {
    let tokens = token_iter.take(num).collect::<Vec<Token>>();
    if tokens.len() == num {
        Ok(tokens)
    } else {
        Err(Error::MissingArguments(num - tokens.len()))
    }
}

fn expect_key(arguments: &mut Vec<Token>) -> Result<Key, Error> {
    if let Token::String(vec) = arguments.remove(0) {
        return Ok(Key(vec));
    } else {
        return Err(Error::KeyNotFound);
    }
}

fn expect_binary(arguments: &mut Vec<Token>) -> Result<Vec<u8>, Error> {
    let first = arguments.remove(0);

    if let Token::String(vec) = first {
        return Ok(vec);
    } else {
        return Err(Error::UnexpectedToken {
            expect: Token::String(vec![]),
            found: Some(first),
        });
    }
}

#[derive(Debug)]
pub enum Token {
    String(Vec<u8>),
    Array(usize),
}

pub struct TokenIter<T: Read>(BufReader<T>);

impl<T: Read> TokenIter<T> {
    fn consume_bytes(&mut self, amount: usize) -> Result<Vec<u8>, Error> {
        let mut buffer: Vec<u8> = vec![0; amount];
        self.0
            .read_exact(&mut buffer)
            .map_err(|err| -> _ { Error::IoError(err) })?;

        Ok(buffer)
    }

    // Omits the /r and /n char
    fn consume_line(&mut self) -> Result<Vec<u8>, Error> {
        let mut buffer = vec![];
        self.0
            .read_until(10, &mut buffer)
            .map_err(|err| -> _ { Error::IoError(err) })?;

        if let &[13, 10] = &buffer[buffer.len() - 2..] {
            buffer.pop();
            buffer.pop();
            Ok(buffer)
        } else {
            Err(Error::MissingCrlf)
        }
    }

    fn next_token(&mut self) -> Result<Token, Error> {
        let prefix = self.consume_bytes(1)?;

        match prefix[0] {
            // $
            36 => {
                let line = self.consume_line()?;
                let bulk_string_len = bytes_to_integer(line);

                // Consume the length of the string plus following /r/n
                let string = self.consume_bytes(bulk_string_len)?;
                self.consume_bytes(2)?;
                return Ok(Token::String(string));
            }
            // *
            42 => {
                let line = self.consume_line().unwrap();
                let num_of_items = bytes_to_integer(line);
                return Ok(Token::Array(num_of_items));
            }
            _ => unreachable!(),
        }
    }
}

impl<T: Read> Iterator for TokenIter<T> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_token() {
            Ok(token) => Some(token),
            Err(error) => {
                println!("Encounter error while emiting new token. Error: {error:?}");
                None
            }
        }
    }
}

fn bytes_to_integer(bytes: Vec<u8>) -> usize {
    str::from_utf8(&bytes).unwrap().parse::<usize>().unwrap()
}

fn bytes_to_string(bytes: Vec<u8>) -> String {
    String::from(str::from_utf8(&bytes).unwrap())
}

#[cfg(test)]
mod tests {
    use crate::connection::inbound::CommandIter;
    use crate::connection::mock_tcp_stream::MockTcpStream;
    use crate::core::Command;

    #[test]
    fn get() {
        let key = "key";
        let input = format!("*2\r\n$3\r\nGET\r\n$3\r\n{}\r\n", key);
        let stream = MockTcpStream::new(input.as_bytes());

        let mut command_iter = CommandIter::new(stream);
        if let Some(Command::Get(key)) = command_iter.next() {
            assert_eq!(key.0, "key".as_bytes());
        } else {
            panic!("Failed to parse command");
        }
    }

    #[test]
    fn set() {
        let key = "key";
        let value = "123";
        let input = format!("*3\r\n$3\r\nSET\r\n$3\r\n{}\r\n$3\r\n{}\r\n", key, value);
        let stream = MockTcpStream::new(input.as_bytes());

        let mut command_iter = CommandIter::new(stream);
        if let Some(Command::Set(key, value)) = command_iter.next() {
            assert_eq!(key.0, "key".as_bytes());
            assert_eq!(value, "123".as_bytes());
        } else {
            panic!("Failed to parse command");
        }
    }
}
