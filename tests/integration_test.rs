mod common;
extern crate rand;
extern crate redis;
use rand::{distributions::Alphanumeric, Rng};
use redis::Commands;
use serial_test::serial;
use std::thread;
use std::time::Duration;

#[test]
#[serial]
fn flush() {
    let mut conn = common::setup();
    let key1 = random_key();
    let key2 = random_key();
    let key3 = random_key();
    let _result: String = conn
        .set_multiple(&[(&key1, "123"), (&key2, "456"), (&key3, "789")])
        .unwrap();

    let result: Option<String> = redis::cmd("FLUSHALL").query(&mut conn).unwrap();
    assert_string(result, "OK");

    let result: Vec<Option<String>> = conn.mget(vec![&key1, &key2, &key3]).unwrap();
    assert_eq!(result, vec![None; 3]);
}

#[test]
#[serial]
fn get_and_set() {
    let mut conn = common::setup();
    let key = random_key();

    let result: Option<String> = conn.get(&key).unwrap();
    assert_eq!(result, None);

    let result: Option<String> = conn.set(&key, "123").unwrap();
    assert_string(result, "OK");

    let result: Option<String> = conn.get(&key).unwrap();
    assert_string(result, "123");

    let result: Option<String> = conn.getset(&key, "456").unwrap();
    assert_string(result, "123");
}

#[test]
#[serial]
fn set_nx() {
    let mut conn = common::setup();
    let key = random_key();

    let result: usize = conn.set_nx(&key, "123").unwrap();
    assert_eq!(result, 1);

    let result: usize = conn.set_nx(&key, "456").unwrap();
    assert_eq!(result, 0);
}

#[test]
#[serial]
fn m_get_and_m_set() {
    let mut conn = common::setup();
    let key1 = random_key();
    let key2 = random_key();

    let result: Vec<Option<String>> = conn.mget(vec![&key1, &key2]).unwrap();
    assert_eq!(result, vec![None, None]);

    let result: String = conn
        .set_multiple(&[(&key1, "123"), (&key2, "456")])
        .unwrap();

    assert_string(Some(result), "OK");

    let result: Vec<Option<String>> = conn.mget(vec![&key1, &key2]).unwrap();
    assert_eq!(
        result,
        vec![Some(String::from("123")), Some(String::from("456"))]
    );
}

#[test]
#[serial]
fn del_and_get_del() {
    let mut conn = common::setup();
    let key1 = random_key();
    let key2 = random_key();

    set(&mut conn, &key1, "123");
    let result: usize = conn.del(vec![&key1, &key2]).unwrap();
    assert_eq!(result, 1);

    set(&mut conn, &key1, "123");
    let result: Option<String> = conn.get_del(&key1).unwrap();
    assert_string(result, "123");
    let result: Option<String> = conn.get_del(&key2).unwrap();
    assert_eq!(result, None);
}

#[test]
#[serial]
fn exists() {
    let mut conn = common::setup();
    let key1 = random_key();
    let key2 = random_key();

    let result: usize = conn.exists(vec![&key1, &key2]).unwrap();
    assert_eq!(result, 0);

    set(&mut conn, &key1, "123");
    set(&mut conn, &key2, "456");
    let result: usize = conn.exists(vec![&key1, &key2]).unwrap();
    assert_eq!(result, 2);
}

#[test]
#[serial]
fn expire() {
    let mut conn = common::setup();
    let key = random_key();

    let result: isize = conn.ttl(&key).unwrap();
    assert_eq!(result, -2);

    set(&mut conn, &key, "123");
    let result: isize = conn.ttl(&key).unwrap();
    assert_eq!(result, -1);

    let result: usize = conn.expire(&key, 2).unwrap();
    assert_eq!(result, 1);

    let result: isize = conn.ttl(&key).unwrap();
    assert_eq!(result, 1);

    thread::sleep(Duration::from_secs(3));
    let result: Option<String> = conn.get(&key).unwrap();
    assert_eq!(result, None);
}

#[test]
#[serial]
fn incr_decr() {
    let mut conn = common::setup();
    let key = random_key();

    let result: isize = conn.incr(&key, 1).unwrap();
    assert_eq!(result, 1);

    let result: isize = conn.incr(&key, 2).unwrap();
    assert_eq!(result, 3);

    let result: isize = conn.decr(&key, 5).unwrap();
    assert_eq!(result, -2);

    let result: isize = conn.get(&key).unwrap();
    assert_eq!(result, -2);

    set(&mut conn, &key, "abc");
    assert!(conn.incr::<String, i32, String>(key.clone(), 1).is_err());
    assert!(conn.decr::<String, i32, String>(key, 1).is_err());
}

fn set(conn: &mut redis::Connection, key: &str, value: &str) {
    let _result: Option<String> = conn.set(key, value).unwrap();
}

fn assert_string(result: Option<String>, desire: &str) {
    assert_eq!(result, Some(String::from(desire)));
}

fn random_key() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect()
}
