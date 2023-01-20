use std::net::TcpStream;
use tiny_redis::server::Server;
extern crate redis;

pub fn setup() {
    let endpoint = Server {
        interface: String::from("127.0.0.1"),
        port: 7878,
    };

    endpoint.start();
}

pub fn connect() -> redis::Connection {
    let client = redis::Client::open("redis://127.0.0.1:7878/").unwrap();
    client.get_connection().unwrap()
}
