//! Experimental Bedrock combo timing assist.
//! Recognizes Space presses made while Ctrl+W were recently held, allowing
//! imperfect near-simultaneous release of both movement keys.
//! NEVER intercepts or synthesizes W/Ctrl/Space; normal left CPS keeps running.
//! A single added click at the estimated falling phase is only a best effort,
//! not a guaranteed critical (real vertical velocity/server rules are unknown).

use super::{EngineConfig, EngineSignals};
use crate::os;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub fn run(sig: Arc<EngineSignals>, cfg: Arc<Mutex<EngineConfig>>) {
    let mut space_was = true;
    let mut w_recent: Option<Instant> = None;
    let mut ctrl_recent: Option<Instant> = None;
    let mut jump: Option<(Instant, u64, u64)> = None;
    let mut click_seen = false;
    let mut latest_click_seq = os::physical_lmb_press_seq();
    let mut last_auto_seq = sig.left_click_seq.load(Ordering::Relaxed);

    while sig.running.load(Ordering::Relaxed) {
        let (bc, left_enabled) = {
            let c = cfg.lock().unwrap();
            (c.bettercrits.clone(), c.left.enabled)
        };
        let allowed = bc.enabled
            && !sig.panic.load(Ordering::Relaxed)
            && !sig.capturing.load(Ordering::Relaxed)
            && os::is_minecraft_active()
            && !os::cursor_visible();

        let space = os::key_held(0x20);
        let w = os::physical_forward_held();
        let ctrl = os::physical_ctrl_held();
        let phys_seq = os::physical_lmb_press_seq();
        let auto_seq = sig.left_click_seq.load(Ordering::Relaxed);
        if !allowed {
            jump = None;
            click_seen = false;
            w_recent = None;
            ctrl_recent = None;
            space_was = space;
            latest_click_seq = phys_seq;
            last_auto_seq = auto_seq;
            thread::sleep(Duration::from_millis(8));
            continue;
        }

        if w { w_recent = Some(Instant::now()); }
        if ctrl { ctrl_recent = Some(Instant::now()); }
        let combo_tolerance = Duration::from_millis(160);
        // The user often releases W/Ctrl on the SAME moment they press Space.
        // Sample their recent hold history; do not require them to still be down.
        if space && !space_was {
            let had_w = w_recent.is_some_and(|t| t.elapsed() <= combo_tolerance);
            let had_ctrl = ctrl_recent.is_some_and(|t| t.elapsed() <= combo_tolerance);
            if had_w && had_ctrl {
                jump = Some((Instant::now(), phys_seq, auto_seq));
                click_seen = os::physical_button_held(true);
            }
        }
        space_was = space;

        if let Some((start, click_start, auto_start)) = jump {
            click_seen |= os::physical_button_held(true)
                || phys_seq != click_start
                || auto_seq != auto_start;
            // One extra click after the apex, if the user requested an attack.
            // No forced pauses for the existing autoclicker.
            let delay = Duration::from_millis(bc.delay_ms.clamp(200.0, 520.0) as u64);
            if start.elapsed() >= delay {
                if click_seen && left_enabled
                    && os::is_minecraft_active()
                    && !os::cursor_visible()
                {
                    os::timed_crit_tap();
                }
                jump = None;
                click_seen = false;
            }
        }
        latest_click_seq = phys_seq;
        last_auto_seq = auto_seq;
        // Suppress unused variable warnings while retaining edge snapshots.
        let _ = (latest_click_seq, last_auto_seq);
        thread::sleep(Duration::from_millis(3));
    }
}
