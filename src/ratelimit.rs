use std::{sync::Mutex, time::Duration};
use web_time::Instant;

use crate::util::sleep;

/// Ratelimit that enforces a minimum delay between initiating requests.
///
/// Works in wasm and desktop environments.
pub struct Ratelimit {
    delay: Duration,
    next_available: Mutex<Option<Instant>>,
}

impl Ratelimit {
    pub const fn new(delay: Duration) -> Self {
        Self {
            delay,
            next_available: Mutex::new(None),
        }
    }
    pub async fn wait(&self) {
        loop {
            let sleep_time;
            {
                let mut next_available = self.next_available.lock().unwrap();
                let next_available = next_available.get_or_insert_with(Instant::now);
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
