use std::io::{Error, Read};

// This module facilitates testing
pub struct MockTcpStream {
    inner_buffer: Vec<u8>,
}

impl MockTcpStream {
    pub fn new(bytes: &[u8]) -> Self {
        MockTcpStream {
            inner_buffer: bytes.to_vec(),
        }
    }
}

impl Read for MockTcpStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let len = self.inner_buffer.len();
        buf[..len].clone_from_slice(&self.inner_buffer);
        Ok(len)
    }
}

mod tests {
    use super::MockTcpStream;
    use std::io::Read;
    use std::str;

    #[test]
    fn basic() {
        let input_bytes = b"*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n";
        let mut stream = MockTcpStream::new(input_bytes);

        let mut buffer: Vec<u8> = vec![0; input_bytes.len()];
        stream
            .read_exact(&mut buffer)
            .expect("Fail to read from MockTcpStream");

        let output = str::from_utf8(&buffer).unwrap();
        assert_eq!(output, "*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n");
    }
}
