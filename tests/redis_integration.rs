//! Simple integration tests that require a running Redis (via Docker or local).
//!
//! These use the `redis` crate directly to validate that a Redis instance is reachable
//! and that basic operations work. This is useful for manual verification when
//! developing the EpochServer Redis layer.

use redis::{Client, Commands};

fn get_redis_url() -> String {
    std::env::var("EPOCH_REDIS_URL")
        .or_else(|_| std::env::var("REDIS_URL"))
        .unwrap_or_else(|_| {
            eprintln!(
                "No EPOCH_REDIS_URL or REDIS_URL set. Defaulting to redis://127.0.0.1:6379/0"
            );
            eprintln!("Make sure you ran: docker compose -f docker-compose.redis.yml up -d");
            "redis://127.0.0.1:6379/0".to_string()
        })
}

#[test]
fn test_redis_is_reachable() {
    let url = get_redis_url();
    println!("Connecting to Redis at: {}", url);

    let client = Client::open(url).expect("failed to create redis client");
    let mut con = client.get_connection().expect("failed to connect to redis");

    let pong: String = redis::cmd("PING").query(&mut con).expect("PING failed");
    assert_eq!(pong, "PONG");

    println!("Redis PING successful!");
}

#[test]
fn test_basic_set_get() {
    let url = get_redis_url();
    let client = Client::open(url).expect("redis client");
    let mut con = client.get_connection().expect("redis connection");

    let key = "epochserver:test:key";
    let value = "hello from integration test";

    let _: () = con.set(key, value).expect("SET failed");
    let got: String = con.get(key).expect("GET failed");

    assert_eq!(got, value);
    println!("SET/GET roundtrip successful!");

    // cleanup
    let _: () = con.del(key).expect("del failed");
}

#[test]
fn test_large_value_and_incr() {
    let url = get_redis_url();
    let client = Client::open(url).expect("redis client");
    let mut con = client.get_connection().expect("redis connection");

    let large_value: String = (0..5000).map(|i| ((i % 26) as u8 + b'a') as char).collect();
    let key = "epochserver:test:large";

    let _: () = con.set(key, &large_value).expect("SET large failed");
    let got: String = con.get(key).expect("GET large failed");

    assert_eq!(got.len(), 5000);

    // Test INCR (used by 830)
    let counter_key = "epochserver:test:counter";
    let _: () = con.del(counter_key).expect("del failed");

    let n1: i64 = con.incr(counter_key, 1).expect("INCR failed");
    let n2: i64 = con.incr(counter_key, 1).expect("INCR failed");

    assert_eq!(n1, 1);
    assert_eq!(n2, 2);

    println!("Large value + INCR test passed!");
}
