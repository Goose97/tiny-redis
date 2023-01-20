mod common;
extern crate redis;
use redis::Commands;
use std::thread;

#[test]
fn string_commands() {
    thread::spawn(|| common::setup());
    let mut conn = common::connect();
    let result: Option<String> = conn.get("key").unwrap();
    assert_eq!(result, None);

    let result: Option<String> = conn.set("key", "abc").unwrap();
    assert_eq!(result, Some(String::from("OK")));

    let result: Option<String> = conn.get("key").unwrap();
    assert_eq!(result, Some(String::from("abc")));
}
