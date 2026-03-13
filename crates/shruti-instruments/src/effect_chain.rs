use serde::{Deserialize, Serialize};
use shruti_dsp::AudioBuffer;
use shruti_dsp::effects::{Delay, Reverb};

/// An individual effect slot in an instrument's effect chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstrumentEffectType {
    Chorus,
    Delay,
    Reverb,
    Distortion,
    FilterDrive,
}

/// Settings for an individual effect in the chain.
#[derive(Debug, Clone)]
pub struct InstrumentEffect {
    pub effect_type: InstrumentEffectType,
    pub enabled: bool,
    pub mix: f32,
    state: EffectState,
    /// Pre-allocated dry signal buffer for effects that need dry/wet mixing.
    /// Reused across process() calls to avoid RT allocations.
    dry_buffer: AudioBuffer,
}

/// Internal state for each effect type.
#[derive(Debug, Clone)]
enum EffectState {
    Chorus(ChorusState),
    Delay(Delay),
    Reverb(Reverb),
    Distortion(DistortionState),
    FilterDrive(FilterDriveState),
}

/// Simple chorus using a modulated delay line.
#[derive(Debug, Clone)]
struct ChorusState {
    buffer_l: Vec<f32>,
    buffer_r: Vec<f32>,
    write_pos: usize,
    phase: f32,
    rate: f32,
    depth: f32,
    sample_rate: f32,
}

impl ChorusState {
    fn new(sample_rate: f32) -> Self {
        let buf_size = (sample_rate * 0.05) as usize; // 50ms max delay
        Self {
            buffer_l: vec![0.0; buf_size.max(1)],
            buffer_r: vec![0.0; buf_size.max(1)],
            write_pos: 0,
            phase: 0.0,
            rate: 1.5,
            depth: 0.003, // 3ms modulation depth
            sample_rate,
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        *self = Self::new(sample_rate);
    }

    fn process(&mut self, buffer: &mut AudioBuffer) {
        let frames = buffer.frames();
        let channels = buffer.channels();
        let buf_len = self.buffer_l.len();
        if buf_len == 0 {
            return;
        }

        let base_delay = self.sample_rate * 0.007; // 7ms base delay

        for frame in 0..frames {
            // LFO modulates delay time
            let lfo = (self.phase * std::f32::consts::TAU).sin();
            self.phase += self.rate / self.sample_rate;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }

            let delay_samples = base_delay + lfo * self.depth * self.sample_rate;
            let delay_int = delay_samples as usize;
            let delay_frac = delay_samples - delay_int as f32;

            for ch in 0..channels.min(2) {
                let input = buffer.get(frame, ch);
                let buf = if ch == 0 {
                    &mut self.buffer_l
                } else {
                    &mut self.buffer_r
                };

                buf[self.write_pos % buf_len] = input;

                // Linear interpolation read
                let read_pos1 = (self.write_pos + buf_len - delay_int) % buf_len;
                let read_pos2 = (self.write_pos + buf_len - delay_int - 1) % buf_len;
                let delayed = buf[read_pos1] * (1.0 - delay_frac) + buf[read_pos2] * delay_frac;

                buffer.set(frame, ch, input + delayed);
            }
            self.write_pos = (self.write_pos + 1) % buf_len;
        }
    }

    fn reset(&mut self) {
        self.buffer_l.fill(0.0);
        self.buffer_r.fill(0.0);
        self.write_pos = 0;
        self.phase = 0.0;
    }
}

/// Simple waveshaper distortion.
#[derive(Debug, Clone)]
struct DistortionState {
    pub drive: f32, // 1.0 = clean, higher = more distortion
}

impl DistortionState {
    fn new() -> Self {
        Self { drive: 2.0 }
    }

    fn process(&self, buffer: &mut AudioBuffer) {
        let frames = buffer.frames();
        let channels = buffer.channels();
        for frame in 0..frames {
            for ch in 0..channels {
                let input = buffer.get(frame, ch);
                let driven = input * self.drive;
                // Soft clipping via tanh
                let output = driven.tanh();
                buffer.set(frame, ch, output);
            }
        }
    }
}

/// Filter drive — saturates signal before a simple one-pole low-pass.
#[derive(Debug, Clone)]
struct FilterDriveState {
    pub drive: f32,
    pub tone: f32, // 0.0 = dark, 1.0 = bright
    prev: [f32; 2],
}

impl FilterDriveState {
    fn new() -> Self {
        Self {
            drive: 1.5,
            tone: 0.5,
            prev: [0.0; 2],
        }
    }

    fn process(&mut self, buffer: &mut AudioBuffer) {
        let frames = buffer.frames();
        let channels = buffer.channels();
        // One-pole coefficient from tone
        let coeff = self.tone.clamp(0.01, 0.99);
        for frame in 0..frames {
            for ch in 0..channels.min(2) {
                let input = buffer.get(frame, ch);
                let driven = (input * self.drive).tanh();
                // One-pole LPF: y = prev + coeff * (x - prev)
                self.prev[ch as usize] += coeff * (driven - self.prev[ch as usize]);
                buffer.set(frame, ch, self.prev[ch as usize]);
            }
        }
    }

    fn reset(&mut self) {
        self.prev = [0.0; 2];
    }
}

impl InstrumentEffect {
    /// Create a new effect of the given type.
    pub fn new(effect_type: InstrumentEffectType, sample_rate: f32) -> Self {
        let state = match &effect_type {
            InstrumentEffectType::Chorus => EffectState::Chorus(ChorusState::new(sample_rate)),
            InstrumentEffectType::Delay => {
                let mut delay = Delay::new(sample_rate);
                delay.time = 0.25;
                delay.feedback = 0.3;
                delay.mix = 0.3;
                EffectState::Delay(delay)
            }
            InstrumentEffectType::Reverb => {
                let mut reverb = Reverb::new(sample_rate);
                reverb.mix = 0.2;
                reverb.room_size = 0.5;
                reverb.damping = 0.5;
                EffectState::Reverb(reverb)
            }
            InstrumentEffectType::Distortion => EffectState::Distortion(DistortionState::new()),
            InstrumentEffectType::FilterDrive => EffectState::FilterDrive(FilterDriveState::new()),
        };
        Self {
            effect_type,
            enabled: true,
            mix: 0.5,
            state,
            dry_buffer: AudioBuffer::new(2, 256),
        }
    }

    /// Process audio through this effect (dry/wet controlled by mix).
    pub fn process(&mut self, buffer: &mut AudioBuffer) {
        if !self.enabled {
            return;
        }

        // Pre-size dry buffer before borrowing self.state (avoids double-borrow).
        let channels = buffer.channels();
        let frames = buffer.frames();
        self.ensure_dry_buffer(channels, frames);

        // For effects that handle mix internally (Delay, Reverb), pass through directly.
        // For others, apply dry/wet mix manually.
        match &mut self.state {
            EffectState::Delay(delay) => {
                delay.mix = self.mix;
                delay.process(buffer);
            }
            EffectState::Reverb(reverb) => {
                reverb.mix = self.mix;
                reverb.process(buffer);
            }
            EffectState::Chorus(chorus) => {
                // Copy dry signal using interleaved slice (vectorizable)
                self.dry_buffer
                    .as_interleaved_mut()
                    .copy_from_slice(buffer.as_interleaved());
                chorus.process(buffer);
                // Crossfade dry/wet using interleaved slices (vectorizable)
                let mix = self.mix;
                let dry_mix = 1.0 - mix;
                let wet = buffer.as_interleaved_mut();
                let dry = self.dry_buffer.as_interleaved();
                for i in 0..wet.len() {
                    wet[i] = dry[i] * dry_mix + wet[i] * mix;
                }
            }
            EffectState::Distortion(dist) => {
                self.dry_buffer
                    .as_interleaved_mut()
                    .copy_from_slice(buffer.as_interleaved());
                dist.process(buffer);
                let mix = self.mix;
                let dry_mix = 1.0 - mix;
                let wet = buffer.as_interleaved_mut();
                let dry = self.dry_buffer.as_interleaved();
                for i in 0..wet.len() {
                    wet[i] = dry[i] * dry_mix + wet[i] * mix;
                }
            }
            EffectState::FilterDrive(fd) => {
                self.dry_buffer
                    .as_interleaved_mut()
                    .copy_from_slice(buffer.as_interleaved());
                fd.process(buffer);
                let mix = self.mix;
                let dry_mix = 1.0 - mix;
                let wet = buffer.as_interleaved_mut();
                let dry = self.dry_buffer.as_interleaved();
                for i in 0..wet.len() {
                    wet[i] = dry[i] * dry_mix + wet[i] * mix;
                }
            }
        }
    }

    /// Ensure the pre-allocated dry buffer matches the required dimensions.
    /// Only reallocates when buffer size changes (rare, not per-call).
    fn ensure_dry_buffer(&mut self, channels: u16, frames: u32) {
        if self.dry_buffer.channels() != channels || self.dry_buffer.frames() != frames {
            self.dry_buffer = AudioBuffer::new(channels, frames);
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        match &mut self.state {
            EffectState::Chorus(c) => c.set_sample_rate(sample_rate),
            EffectState::Delay(d) => d.set_sample_rate(sample_rate),
            EffectState::Reverb(r) => r.set_sample_rate(sample_rate),
            EffectState::Distortion(_) => {}
            EffectState::FilterDrive(_) => {}
        }
    }

    pub fn reset(&mut self) {
        match &mut self.state {
            EffectState::Chorus(c) => c.reset(),
            EffectState::Delay(d) => d.reset(),
            EffectState::Reverb(r) => r.reset(),
            EffectState::Distortion(_) => {}
            EffectState::FilterDrive(fd) => fd.reset(),
        }
    }
}

/// An ordered chain of effects applied to an instrument's output.
#[derive(Debug, Clone)]
pub struct EffectChain {
    effects: Vec<InstrumentEffect>,
    /// Scratch buffer for isolating instrument output before applying effects.
    scratch: AudioBuffer,
    /// Pre-allocated dry signal buffer for effects needing dry/wet mixing.
    /// Avoids heap allocation per effect process call.
    dry_scratch: AudioBuffer,
}

impl EffectChain {
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
            scratch: AudioBuffer::new(2, 256),
            dry_scratch: AudioBuffer::new(2, 256),
        }
    }

    /// Add an effect to the end of the chain.
    pub fn add(&mut self, effect: InstrumentEffect) {
        self.effects.push(effect);
    }

    /// Remove an effect by index.
    pub fn remove(&mut self, index: usize) -> Option<InstrumentEffect> {
        if index < self.effects.len() {
            Some(self.effects.remove(index))
        } else {
            None
        }
    }

    /// Get all effects.
    pub fn effects(&self) -> &[InstrumentEffect] {
        &self.effects
    }

    /// Get all effects mutably.
    pub fn effects_mut(&mut self) -> &mut [InstrumentEffect] {
        &mut self.effects
    }

    /// Number of effects in the chain.
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Whether the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Ensure scratch buffer matches the required dimensions.
    fn ensure_scratch(&mut self, channels: u16, frames: u32) {
        if self.scratch.channels() != channels || self.scratch.frames() != frames {
            self.scratch = AudioBuffer::new(channels, frames);
        }
    }

    /// Process: renders instrument into scratch buffer, applies effect chain,
    /// then adds the result to the output buffer.
    ///
    /// `render_fn` should render the instrument into the provided buffer
    /// (writing, not adding — the buffer starts zeroed).
    pub fn process_with<F>(&mut self, output: &mut AudioBuffer, render_fn: F)
    where
        F: FnOnce(&mut AudioBuffer),
    {
        if self.effects.is_empty() || self.effects.iter().all(|e| !e.enabled) {
            // No active effects — render directly into output (instrument adds)
            render_fn(output);
            return;
        }

        let channels = output.channels();
        let frames = output.frames();
        self.ensure_scratch(channels, frames);

        // Zero the scratch buffer
        self.scratch.clear();

        // Render instrument into scratch
        render_fn(&mut self.scratch);

        // Apply effect chain in order
        for effect in &mut self.effects {
            effect.process(&mut self.scratch);
        }

        // Add processed signal to output
        output.mix_from(&self.scratch);
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        for effect in &mut self.effects {
            effect.set_sample_rate(sample_rate);
        }
    }

    pub fn reset(&mut self) {
        for effect in &mut self.effects {
            effect.reset();
        }
    }
}

impl Default for EffectChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_chain_creation() {
        let chain = EffectChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn effect_chain_add_remove() {
        let mut chain = EffectChain::new();
        chain.add(InstrumentEffect::new(InstrumentEffectType::Chorus, 48000.0));
        chain.add(InstrumentEffect::new(InstrumentEffectType::Delay, 48000.0));
        assert_eq!(chain.len(), 2);

        let removed = chain.remove(0).unwrap();
        assert!(matches!(removed.effect_type, InstrumentEffectType::Chorus));
        assert_eq!(chain.len(), 1);

        assert!(chain.remove(5).is_none());
    }

    #[test]
    fn effect_chain_passthrough_when_empty() {
        let mut chain = EffectChain::new();
        let mut output = AudioBuffer::new(2, 64);

        chain.process_with(&mut output, |buf| {
            for f in 0..buf.frames() {
                buf.set(f, 0, 1.0);
                buf.set(f, 1, 1.0);
            }
        });

        // Should pass through unmodified
        for f in 0..64 {
            assert!((output.get(f, 0) - 1.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn effect_chain_disabled_effects_passthrough() {
        let mut chain = EffectChain::new();
        let mut effect = InstrumentEffect::new(InstrumentEffectType::Distortion, 48000.0);
        effect.enabled = false;
        chain.add(effect);

        let mut output = AudioBuffer::new(2, 64);
        chain.process_with(&mut output, |buf| {
            for f in 0..buf.frames() {
                buf.set(f, 0, 0.5);
                buf.set(f, 1, 0.5);
            }
        });

        for f in 0..64 {
            assert!((output.get(f, 0) - 0.5).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn distortion_changes_signal() {
        let mut effect = InstrumentEffect::new(InstrumentEffectType::Distortion, 48000.0);
        effect.mix = 1.0;
        let mut buf = AudioBuffer::new(2, 64);
        for f in 0..64 {
            buf.set(f, 0, 0.8);
            buf.set(f, 1, 0.8);
        }

        effect.process(&mut buf);

        // Distorted signal should be different from input
        let sample = buf.get(0, 0);
        assert!(
            (sample - 0.8).abs() > 0.01,
            "Distortion should modify signal, got {sample}"
        );
        // tanh(0.8 * 2.0) = tanh(1.6) ≈ 0.9217
        assert!(sample > 0.85 && sample < 0.95);
    }

    #[test]
    fn chorus_produces_output() {
        let mut effect = InstrumentEffect::new(InstrumentEffectType::Chorus, 48000.0);
        effect.mix = 1.0;
        let mut buf = AudioBuffer::new(2, 256);
        // Feed a simple signal
        for f in 0..256 {
            let val = (f as f32 / 256.0 * std::f32::consts::TAU).sin() * 0.5;
            buf.set(f, 0, val);
            buf.set(f, 1, val);
        }

        effect.process(&mut buf);

        // Output should have nonzero content
        let mut has_nonzero = false;
        for f in 0..256 {
            if buf.get(f, 0).abs() > 0.01 {
                has_nonzero = true;
                break;
            }
        }
        assert!(has_nonzero);
    }

    #[test]
    fn reverb_effect_processes() {
        let mut effect = InstrumentEffect::new(InstrumentEffectType::Reverb, 48000.0);
        effect.mix = 0.5;
        // Use a buffer large enough for the comb delay lines (~1200+ samples at 48kHz)
        let mut buf = AudioBuffer::new(2, 4800);
        buf.set(0, 0, 1.0);
        buf.set(0, 1, 1.0);

        effect.process(&mut buf);

        // After impulse, reverb tail should produce nonzero samples in the late portion
        let mut has_tail = false;
        for f in 1300..4800 {
            if buf.get(f, 0).abs() > 0.001 {
                has_tail = true;
                break;
            }
        }
        assert!(has_tail, "Reverb should produce a tail after impulse");
    }

    #[test]
    fn delay_effect_processes() {
        let mut effect = InstrumentEffect::new(InstrumentEffectType::Delay, 48000.0);
        effect.mix = 1.0;
        let mut buf = AudioBuffer::new(2, 48000);
        buf.set(0, 0, 1.0);
        buf.set(0, 1, 1.0);

        effect.process(&mut buf);

        // Delay at 0.25s = 12000 samples, should see echo there
        let echo_sample = buf.get(12000, 0);
        assert!(
            echo_sample.abs() > 0.1,
            "Should have echo at 12000 samples, got {echo_sample}"
        );
    }

    #[test]
    fn filter_drive_changes_signal() {
        let mut effect = InstrumentEffect::new(InstrumentEffectType::FilterDrive, 48000.0);
        effect.mix = 1.0;
        let mut buf = AudioBuffer::new(2, 64);
        for f in 0..64 {
            buf.set(f, 0, 0.7);
            buf.set(f, 1, 0.7);
        }

        effect.process(&mut buf);

        let sample = buf.get(32, 0);
        assert!(
            (sample - 0.7).abs() > 0.01,
            "Filter drive should modify signal, got {sample}"
        );
    }

    #[test]
    fn effect_chain_processes_signal() {
        let mut chain = EffectChain::new();
        let mut dist = InstrumentEffect::new(InstrumentEffectType::Distortion, 48000.0);
        dist.mix = 1.0;
        chain.add(dist);

        let mut output = AudioBuffer::new(2, 64);
        chain.process_with(&mut output, |buf| {
            for f in 0..buf.frames() {
                buf.set(f, 0, 0.8);
                buf.set(f, 1, 0.8);
            }
        });

        let sample = output.get(0, 0);
        assert!(
            (sample - 0.8).abs() > 0.01,
            "Chain with distortion should modify signal"
        );
    }

    #[test]
    fn effect_set_sample_rate() {
        let mut effect = InstrumentEffect::new(InstrumentEffectType::Chorus, 44100.0);
        effect.set_sample_rate(96000.0);
        // Should not panic
        let mut buf = AudioBuffer::new(2, 64);
        effect.process(&mut buf);
    }

    #[test]
    fn effect_reset() {
        let mut effect = InstrumentEffect::new(InstrumentEffectType::Chorus, 48000.0);
        let mut buf = AudioBuffer::new(2, 64);
        for f in 0..64 {
            buf.set(f, 0, 1.0);
        }
        effect.process(&mut buf);
        effect.reset();
        // Should not panic, internal state cleared
    }

    #[test]
    fn effect_chain_set_sample_rate() {
        let mut chain = EffectChain::new();
        chain.add(InstrumentEffect::new(InstrumentEffectType::Chorus, 44100.0));
        chain.add(InstrumentEffect::new(InstrumentEffectType::Delay, 44100.0));
        chain.set_sample_rate(96000.0);
        // Should not panic
    }

    #[test]
    fn dry_wet_mix_at_zero_is_dry() {
        let mut effect = InstrumentEffect::new(InstrumentEffectType::Distortion, 48000.0);
        effect.mix = 0.0;
        let mut buf = AudioBuffer::new(2, 64);
        for f in 0..64 {
            buf.set(f, 0, 0.5);
        }

        effect.process(&mut buf);

        // Mix=0 means fully dry
        for f in 0..64 {
            assert!(
                (buf.get(f, 0) - 0.5).abs() < f32::EPSILON,
                "Mix=0 should be fully dry"
            );
        }
    }
}
