//! Manual check: plays a short typing phrase on the synth packs through the default output
//! device, then exits. `cargo run -p clatterbox-audio --example beep`

use std::sync::Arc;
use std::time::{Duration, Instant};

use clatterbox_audio::{AudioEngine, AudioStatus};
use clatterbox_core::synth::generate;
use clatterbox_core::{
    EngineParams, KeyClass, KeyDir, KeySound, PackSlot, Settings, SynthPreset, Trigger,
};

fn key(class: KeyClass, dir: KeyDir, x: f32) -> Trigger {
    Trigger::key(KeySound { class, dir, x })
}

fn main() {
    let params = Arc::new(EngineParams::from_settings(&Settings::default()));
    let (engine, mut hook) =
        AudioEngine::start(params, Box::new(|s: AudioStatus| println!("status: {s:?}")));
    let deadline = Instant::now() + Duration::from_secs(3);
    while matches!(engine.status(), AudioStatus::Starting) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }

    let phrase = [
        (KeyClass::Default, 0.25),
        (KeyClass::Default, 0.45),
        (KeyClass::Default, 0.60),
        (KeyClass::Space, 0.47),
        (KeyClass::Default, 0.70),
        (KeyClass::Backspace, 0.93),
        (KeyClass::Enter, 0.90),
        (KeyClass::Modifier, 0.08),
    ];
    for preset in SynthPreset::ALL {
        println!("pack: {}", preset.id());
        engine.set_pack(PackSlot::Main, Arc::new(generate(preset, 48_000)));
        for _ in 0..2 {
            for (class, x) in phrase {
                hook.push(key(class, KeyDir::Down, x));
                std::thread::sleep(Duration::from_millis(55));
                hook.push(key(class, KeyDir::Up, x));
                std::thread::sleep(Duration::from_millis(80));
            }
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    let status = engine.status();
    engine.shutdown();
    match status {
        AudioStatus::Running { .. } => println!("ok: {status:?}"),
        other => {
            eprintln!("audio did not run: {other:?}");
            std::process::exit(1);
        }
    }
}
