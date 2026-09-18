//! Manual check of the keyboard hook: `cargo run -p clatterbox-keyhook --example probe [secs]`.
//! PRIVACY: prints only the sound class and the pan position rounded to 0.1, never the key.

use std::sync::Arc;
use std::time::{Duration, Instant};

use clatterbox_core::{EngineParams, Settings, TriggerTx};

fn main() {
    let secs: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let params = Arc::new(EngineParams::from_settings(&Settings::default()));
    let (tx, mut rx) = rtrb::RingBuffer::new(256);
    let hook = clatterbox_keyhook::start(
        TriggerTx(tx),
        params,
        Box::new(|s| println!("status: {s:?}")),
    );
    let end = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < end {
        while let Ok(t) = rx.pop() {
            println!("{} {:.1}", t.sound.class.as_str(), t.sound.x);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    hook.stop();
    println!("stopped");
}
