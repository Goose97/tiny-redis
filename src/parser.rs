use std::{
    borrow::Borrow,
    io,
    io::{BufRead, BufReader, Read},
    str,
};

use super::core::{Command, Key};

pub struct Parser<T: Read>(pub T);

#[derive(Debug)]
pub enum Error {
    KeyNotFound,
    UnexpectedToken { expect: Token, found: Token },
    MissingArguments(usize),
    MissingCrlf,
    IoError(io::Error),
}

impl<T: Read> Parser<T> {
    pub fn parse(self) -> Result<Command, Error> {
        let mut token_iter = self.into_token_iter();

        let command_size = command_size(&mut token_iter);
        let command = command(&mut token_iter)?;
        let mut arguments = arguments(&mut token_iter, command_size - 1)?;

        match command.borrow() {
            "GET" => {
                let key = expect_key(&mut arguments)?;
                return Ok(Command::Get(key));
            }

            "SET" => {
                let key = expect_key(&mut arguments)?;
                let value = expect_binary(&mut arguments)?;
                return Ok(Command::Set(key, value));
            }

            _ => unimplemented!(),
        }
    }

    fn into_token_iter(self) -> TokenIter<T> {
        TokenIter(BufReader::new(self.0))
    }
}

fn command_size<T: Read>(token_iter: &mut TokenIter<T>) -> usize {
    let token = token_iter.next();

    if let Some(Token::Array(size)) = token {
        size
    } else {
        panic!("Expected be array size");
    }
}

fn command<T: Read>(token_iter: &mut TokenIter<T>) -> Result<String, Error> {
    let token = token_iter.next();
    if let Some(Token::String(command)) = token {
        Ok(bytes_to_string(command))
    } else {
        Err(Error::UnexpectedToken {
            expect: Token::String(vec![]),
            found: token.unwrap(),
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
    if let Token::String(vec) = arguments.swap_remove(0) {
        return Ok(Key(vec));
    } else {
        return Err(Error::KeyNotFound);
    }
}

fn expect_binary(arguments: &mut Vec<Token>) -> Result<Vec<u8>, Error> {
    let first = arguments.swap_remove(0);

    if let Token::String(vec) = first {
        return Ok(vec);
    } else {
        return Err(Error::UnexpectedToken {
            expect: Token::String(vec![]),
            found: first,
        });
    }
}

#[derive(Debug)]
pub enum Token {
    String(Vec<u8>),
    Integer(isize),
    Array(usize),
    Error(String),
}

struct TokenIter<T: Read>(BufReader<T>);

impl<T: Read> TokenIter<T> {
    fn consume_bytes(&mut self, amount: usize) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![0; amount];
        self.0.read_exact(&mut buffer).unwrap();
        buffer
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
}

impl<T: Read> Iterator for TokenIter<T> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let prefix = self.consume_bytes(1);

        match prefix[0] {
            // $
            36 => {
                let line = self.consume_line().unwrap();
                let bulk_string_len = bytes_to_integer(line);

                // Consume the length of the string plus following /r/n
                let string = self.consume_bytes(bulk_string_len);
                self.consume_bytes(2);
                return Some(Token::String(string));
            }
            // *
            42 => {
                let line = self.consume_line().unwrap();
                let num_of_items = bytes_to_integer(line);
                return Some(Token::Array(num_of_items));
            }
            _ => unreachable!(),
        }
    }
}

fn bytes_to_integer(bytes: Vec<u8>) -> usize {
    str::from_utf8(&bytes).unwrap().parse::<usize>().unwrap()
}

fn bytes_to_string(bytes: Vec<u8>) -> String {
    String::from(str::from_utf8(&bytes).unwrap())
}

mod tests {
    use crate::core::Command;
    use crate::mock_tcp_stream::MockTcpStream;
    use crate::parser::Parser;

    #[test]
    fn get() {
        let key = "key";
        let input = format!("*2\r\n$3\r\nGET\r\n$3\r\n{}\r\n", key);
        let stream = MockTcpStream::new(input.as_bytes());

        let parser = Parser(stream);
        if let Ok(Command::Get(key)) = parser.parse() {
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

        let parser = Parser(stream);
        if let Ok(Command::Set(key, value)) = parser.parse() {
            assert_eq!(key.0, "key".as_bytes());
            assert_eq!(value, "123".as_bytes());
        } else {
            panic!("Failed to parse command");
        }
    }
}
