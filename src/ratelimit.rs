use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use web_time::Instant;

use crate::util::sleep;

/// Ratelimiter that enforces a minimum delay between initiating requests.
///
/// Works in wasm and desktop environments.
#[derive(Clone)]
pub struct Ratelimiter {
    delay: Duration,
    next_available: Arc<Mutex<Instant>>,
}

impl Ratelimiter {
    pub fn new(delay: Duration) -> Self {
        Self {
            delay,
            next_available: Arc::new(Mutex::new(Instant::now())),
        }
    }
    pub async fn wait(&self) {
        loop {
            let sleep_time;
            {
                let mut next_available = self.next_available.lock().unwrap();
                let now = Instant::now();
                if now >= *next_available {
                    *next_available = now + self.delay;
                    return;
                }
                sleep_time = *next_available - now;
            }
            sleep(sleep_time).await;
        }
    }
}
