//! Sample-accurate audio clock — re-exported from dhvani.

pub use dhvani::clock::AudioClock;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_new() {
        let clock = AudioClock::new(48000);
        assert_eq!(clock.sample_rate, 48000);
        assert_eq!(clock.position_samples, 0);
        assert!(!clock.running);
    }

    #[test]
    fn clock_with_tempo() {
        let clock = AudioClock::with_tempo(48000, 120.0);
        assert_eq!(clock.tempo_bpm, 120.0);
    }

    #[test]
    fn clock_advance() {
        let mut clock = AudioClock::new(48000);
        clock.start();
        clock.advance(48000);
        assert!((clock.position_secs() - 1.0).abs() < 0.001);
    }

    #[test]
    fn clock_seek() {
        let mut clock = AudioClock::new(48000);
        clock.seek_secs(2.5);
        assert!((clock.position_secs() - 2.5).abs() < 0.001);
    }

    #[test]
    fn clock_pts() {
        let mut clock = AudioClock::new(48000);
        clock.seek_secs(1.0);
        let pts = clock.pts_us();
        assert!((pts as f64 - 1_000_000.0).abs() < 100.0);
    }

    #[test]
    fn clock_beats() {
        let mut clock = AudioClock::with_tempo(48000, 120.0);
        clock.start();
        // At 120 BPM, 1 second = 2 beats
        clock.advance(48000);
        let beats = clock.position_beats().unwrap();
        assert!((beats - 2.0).abs() < 0.01);
    }
}
