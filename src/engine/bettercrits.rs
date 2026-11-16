//! Experimental external critical-hit assist for Minecraft Bedrock.
//! Never attacks on its own: requires Ctrl+W, a fresh Space press, and a physical LMB request.
//! Timing is approximate because the client cannot read the player's vertical velocity.

use super::{EngineConfig, EngineSignals};
use crate::os;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

fn release(sig: &EngineSignals) {
    os::set_crit_capture(false);
    sig.crit_pause.store(false, Ordering::Relaxed);
}

pub fn run(sig: Arc<EngineSignals>, cfg: Arc<Mutex<EngineConfig>>) {
    let mut space_was = true;
    let mut armed: Option<Instant> = None;
    let mut attack_requested = false;

    while sig.running.load(Ordering::Relaxed) {
        let bc = { cfg.lock().unwrap().bettercrits.clone() };
        // Never intercept clicks outside the game or in its menus.
        let allowed = bc.enabled
            && !sig.panic.load(Ordering::Relaxed)
            && !sig.capturing.load(Ordering::Relaxed)
            && os::is_minecraft_active()
            && !os::cursor_visible();

        if !allowed {
            if armed.take().is_some() {
                release(&sig);
            }
            space_was = os::key_held(0x20);
            thread::sleep(Duration::from_millis(8));
            continue;
        }

        let space = os::key_held(0x20);
        if space && !space_was && os::key_held(0x57) && os::key_held(0x11) {
            // One attempt per fresh jump. Ctrl+W may remain physically held throughout.
            armed = Some(Instant::now());
            attack_requested = os::physical_button_held(true);
            sig.crit_pause.store(true, Ordering::Relaxed);
            os::set_crit_capture(true);
            os::click_up(true); // stop any ongoing auto-click press
        }
        space_was = space;

        if let Some(start) = armed {
            attack_requested |= os::take_crit_request();
            let elapsed = start.elapsed();
            let delay = Duration::from_millis(bc.delay_ms.clamp(200.0, 520.0) as u64);
            let end = delay + Duration::from_millis(bc.window_ms.clamp(50.0, 300.0) as u64);

            if elapsed >= delay && attack_requested {
                // Experimental sprint interruption: briefly release forward movement.
                // The keyboard hook tracks the physical W state, ignoring injected events.
                if bc.sprint_reset {
                    os::key_up(0x57);
                    thread::sleep(Duration::from_millis(45));
                }
                if os::is_minecraft_active() && !os::cursor_visible() {
                    os::click_down(true);
                    thread::sleep(Duration::from_millis(24));
                    os::click_up(true);
                }
                if bc.sprint_reset && os::is_minecraft_active() && os::physical_forward_held() {
                    os::key_down(0x57);
                }
                // Keep the normal clicker paused briefly to avoid an immediate second hit.
                thread::sleep(Duration::from_millis(65));
                armed = None;
                release(&sig);
            } else if elapsed >= end {
                armed = None;
                release(&sig);
            }
        }

        thread::sleep(Duration::from_millis(3));
    }

    release(&sig);
}
