/// into binary as per RESP protocol (https://redis.io/docs/reference/protocol-spec/)
/// This module provides interface to encode command response
use crate::core::CommandResponse;

pub fn encode(response: CommandResponse) -> Vec<u8> {
    let mut vec = vec![];
    match response {
        // +OK\r\n
        CommandResponse::SimpleString(mut bytes) => {
            vec.extend_from_slice(b"+");
            vec.append(&mut bytes);
            vec.extend_from_slice(b"\r\n");
        }
        // $4\r\nBULK\r\n
        CommandResponse::BulkString(mut bytes) => {
            let content = format!("${}\r\n", bytes.len());
            vec.extend_from_slice(content.as_bytes());
            vec.append(&mut bytes);
            vec.extend_from_slice(b"\r\n");
        }
        // :1000\r\n
        CommandResponse::Integer(integer) => {
            vec.extend_from_slice(b":");
            vec.extend_from_slice(integer.to_string().as_bytes());
            vec.extend_from_slice(b"\r\n");
        }
        // -ERROR\r\n
        CommandResponse::Error(string) => {
            vec.extend_from_slice(b"-");
            vec.extend_from_slice(string.as_bytes());
            vec.extend_from_slice(b"\r\n");
        }
        // $-1\r\n
        CommandResponse::Null => {
            vec.extend_from_slice(b"$-1\r\n");
        }
        // "*2\r\n$5\r\nHello\r\n$5\r\nWorld\r\n"
        CommandResponse::Array(items) => {
            let content = format!("*{}\r\n", items.len());
            vec.extend_from_slice(content.as_bytes());

            for item in items {
                let mut encoded = encode(item);
                vec.append(&mut encoded);
            }
        }
        // *-1\r\n
        CommandResponse::NullArray => {
            vec.extend_from_slice(b"*-1\r\n");
        }
    }

    vec
}

#[cfg(test)]
mod tests {
    use super::encode;
    use crate::core::CommandResponse;

    #[test]
    fn simple_string() {
        let response = CommandResponse::SimpleString(b"Hello World".to_vec());
        assert_eq!(encode(response), b"+Hello World\r\n");
    }

    #[test]
    fn bulk_string() {
        let response = CommandResponse::BulkString(b"Hello World".to_vec());
        assert_eq!(encode(response), b"$11\r\nHello World\r\n");
    }

    #[test]
    fn integer() {
        let response = CommandResponse::Integer(1024);
        assert_eq!(encode(response), b":1024\r\n");
    }

    #[test]
    fn error() {
        let response = CommandResponse::Error(String::from("Goodbye World"));
        assert_eq!(encode(response), b"-Goodbye World\r\n");
    }

    #[test]
    fn null() {
        let response = CommandResponse::Null;
        assert_eq!(encode(response), b"$-1\r\n");
    }

    #[test]
    fn array() {
        let vec = vec![
            CommandResponse::SimpleString(b"Hello World".to_vec()),
            CommandResponse::BulkString(b"Hello World".to_vec()),
            CommandResponse::Integer(1024),
            CommandResponse::Error(String::from("Goodbye World")),
            CommandResponse::Null,
        ];
        let response = CommandResponse::Array(vec);
        assert_eq!(
            encode(response),
            b"*5\r\n+Hello World\r\n$11\r\nHello World\r\n:1024\r\n-Goodbye World\r\n$-1\r\n"
        );
    }

    #[test]
    fn null_array() {
        let response = CommandResponse::NullArray;
        assert_eq!(encode(response), b"*-1\r\n");
    }
}
