//! Experimental critical-hit timing helper for Minecraft Bedrock.
//! Keeps the regular LEFT CLICK autoclicker running at its chosen CPS.
//! Temporarily interrupts Ctrl+W after a manual jump, then restores it.
//! Timing is estimated: there is no access to actual on-ground/vertical velocity state.

use super::{EngineConfig, EngineSignals};
use crate::os;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub fn run(sig: Arc<EngineSignals>, cfg: Arc<Mutex<EngineConfig>>) {
    let mut space_was = true;
    let mut reset_started_at: Option<Instant> = None;
    let mut forward_restored = false;

    while sig.running.load(Ordering::Relaxed) {
        let bc = { cfg.lock().unwrap().bettercrits.clone() };
        let focused = os::is_minecraft_active() && !os::cursor_visible();
        let allowed = bc.enabled
            && !sig.panic.load(Ordering::Relaxed)
            && !sig.capturing.load(Ordering::Relaxed)
            && focused;

        if !allowed {
            if reset_started_at.take().is_some() {
                os::finish_sprint_interrupt(focused);
            }
            forward_restored = false;
            space_was = os::key_held(0x20);
            thread::sleep(Duration::from_millis(8));
            continue;
        }

        let space = os::key_held(0x20);
        // Start at a manual Space press while the player is actually sprinting.
        // The clicker remains untouched. If sprint reset is OFF, this thread does
        // nothing other than polling for the next jump.
        if bc.sprint_reset && space && !space_was
            && os::physical_forward_held()
            && os::physical_ctrl_held()
            && reset_started_at.is_none()
        {
            if os::start_sprint_interrupt() {
                reset_started_at = Some(Instant::now());
                forward_restored = false;
            }
        }
        space_was = space;

        if let Some(start) = reset_started_at {
            // Stop player movement briefly, then resume forward motion without
            // Ctrl so the autoclicker can hit during the estimated falling phase.
            // Finally restore sprint if the user still physically holds both.
            let break_ms = bc.window_ms.clamp(20.0, 80.0);
            let restore_ms = bc.delay_ms.clamp(200.0, 520.0);
            if !forward_restored && start.elapsed() >= Duration::from_millis(break_ms as u64) {
                if os::physical_forward_held() {
                    os::restore_forward_after_interrupt();
                }
                forward_restored = true;
            }
            if start.elapsed() >= Duration::from_millis(restore_ms as u64)
                || !os::physical_forward_held()
                || !os::physical_ctrl_held()
            {
                os::finish_sprint_interrupt(focused);
                reset_started_at = None;
                forward_restored = false;
            }
        }
        thread::sleep(Duration::from_millis(3));
    }
    if reset_started_at.is_some() {
        os::finish_sprint_interrupt(os::is_minecraft_active());
    }
}
