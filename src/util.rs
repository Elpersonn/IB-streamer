// Utility functions and such
use rand::{distributions::Alphanumeric, prelude::*};
use std::time;
#[derive(PartialEq)]
pub enum MessageType {
    KEYCHANGE(String),
    STOPSTREAM, // no data
    STARTSTREAM
}


pub fn generateCookie() -> String {
    let mut rng= rand::thread_rng();
    rng.sample_iter(Alphanumeric).take(32).map(char::from).collect()
}
pub fn timeNow() -> u64 {
    time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap().as_secs()
}
#[cfg(debug_assertions)]
pub fn get_workdir() -> String {
    std::env::var("CARGO_MANIFEST").unwrap()
}

#[cfg(not(debug_assertions))]
pub fn get_workdir() -> String {
    let dir = *std::env::current_dir().unwrap().to_string_lossy();
}