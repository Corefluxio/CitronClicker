//! Safe BetterCrits placeholder.
//! Do not override the player's physically held movement keys or interrupt LMB.
//! Exact critical-hit timing needs an actual Bedrock falling-state signal; a
//! fixed delay from Space cannot guarantee critical hits.

use super::{EngineConfig, EngineSignals};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub fn run(sig: Arc<EngineSignals>, _cfg: Arc<Mutex<EngineConfig>>) {
    while sig.running.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(30));
    }
}
