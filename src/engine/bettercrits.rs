//! Experimental passive BetterCrits assistant for Minecraft Bedrock.
//! IMPORTANT: never captures, delays, or synthesizes mouse attacks.
//! The regular LEFT CLICK engine keeps running at its chosen CPS while jumping.
//! This only schedules an optional short sprint-reset pulse near a guessed falling phase.

use super::{EngineConfig, EngineSignals};
use crate::os;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub fn run(sig: Arc<EngineSignals>, cfg: Arc<Mutex<EngineConfig>>) {
    let mut space_was = true;
    let mut press_was = true;
    let mut jump_at: Option<Instant> = None;
    let mut attacking = false;
    let mut last_click_seq = sig.left_click_seq.load(Ordering::Relaxed);

    while sig.running.load(Ordering::Relaxed) {
        let bc = { cfg.lock().unwrap().bettercrits.clone() };
        let focused = os::is_minecraft_active() && !os::cursor_visible();
        let allowed = bc.enabled
            && !sig.panic.load(Ordering::Relaxed)
            && !sig.capturing.load(Ordering::Relaxed)
            && focused;

        if !allowed {
            jump_at = None;
            attacking = false;
            space_was = os::key_held(0x20);
            press_was = os::physical_button_held(true);
            last_click_seq = sig.left_click_seq.load(Ordering::Relaxed);
            thread::sleep(Duration::from_millis(8));
            continue;
        }

        let space = os::key_held(0x20);
        let phys_lmb = os::physical_button_held(true);
        let current_seq = sig.left_click_seq.load(Ordering::Relaxed);
        if space && !space_was && os::key_held(0x57) {
            // Fresh jump with forward movement. No change to the clicker state.
            jump_at = Some(Instant::now());
            attacking = phys_lmb;
        }
        space_was = space;

        if jump_at.is_some() {
            attacking |= phys_lmb || (!press_was && phys_lmb) || current_seq != last_click_seq;
        }
        press_was = phys_lmb;
        last_click_seq = current_seq;

        if let Some(start) = jump_at {
            let delay = Duration::from_millis(bc.delay_ms.clamp(200.0, 520.0) as u64);
            if start.elapsed() >= delay {
                // Crits can work while sprinting in vanilla Bedrock; this reset is
                // optional for servers with different timing/knockback behavior.
                // We DO NOT pause, intercept, or add any LMB click.
                if bc.sprint_reset
                    && attacking
                    && os::physical_forward_held()
                    && os::is_minecraft_active()
                {
                    os::key_up(0x57);
                    thread::sleep(Duration::from_millis(
                        bc.window_ms.clamp(20.0, 80.0) as u64,
                    ));
                    if os::physical_forward_held() && os::is_minecraft_active() {
                        os::key_down(0x57);
                    }
                }
                jump_at = None;
                attacking = false;
            }
        }
        thread::sleep(Duration::from_millis(3));
    }
}
