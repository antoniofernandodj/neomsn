//! Synthesized sound effects — no audio assets needed.

use std::time::Duration;
use rodio::source::{SineWave, Source};

/// Classic MSN nudge buzz: a short low-frequency warble. Plays on a detached
/// thread; silently does nothing when no audio device is available.
pub fn nudge() {
    std::thread::spawn(|| {
        let Ok((_stream, handle)) = rodio::OutputStream::try_default() else {
            return;
        };
        let Ok(sink) = rodio::Sink::try_new(&handle) else {
            return;
        };
        for _ in 0..3 {
            for freq in [220.0, 180.0] {
                sink.append(
                    SineWave::new(freq)
                        .take_duration(Duration::from_millis(70))
                        .amplify(0.5),
                );
            }
        }
        sink.sleep_until_end();
    });
}
