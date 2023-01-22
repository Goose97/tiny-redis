extern crate redis;
use std::{sync::Once, thread};
use tiny_redis::server::Server;

pub fn setup() -> redis::Connection {
    initialize();
    let client = redis::Client::open("redis://127.0.0.1:7878/").unwrap();
    let mut conn = client.get_connection().unwrap();

    // redis::cmd("FLUSHALL").execute(&mut conn);
    conn
}

static INIT: Once = Once::new();

fn initialize() {
    INIT.call_once(|| {
        thread::spawn(|| {
            let endpoint = Server {
                interface: String::from("127.0.0.1"),
                port: 7878,
            };

            endpoint.start();
        });
    });
}
