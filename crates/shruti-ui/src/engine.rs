use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use shruti_dsp::{AudioBuffer, AudioFormat};

/// Type alias for the audio output callback closure.
type AudioCallback = Box<dyn FnMut(&mut [f32]) + Send + 'static>;
use shruti_engine::AudioStream;
use shruti_engine::backend::{AudioHost, CpalBackend};
use shruti_engine::meter::{SharedMeterLevels, shared_meter_levels};
use shruti_session::RecordingConfig;
use shruti_session::audio_pool::AudioPool;
use shruti_session::track::Track;
use shruti_session::{Session, Timeline, Transport};

/// Shared state between UI and audio thread via atomics.
pub struct SharedTransport {
    /// Current playback position in frames.
    pub position: AtomicU64,
    /// Whether playback is active.
    pub playing: AtomicBool,
    /// Whether recording is active.
    pub recording: AtomicBool,
}

impl Default for SharedTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedTransport {
    pub fn new() -> Self {
        Self {
            position: AtomicU64::new(0),
            playing: AtomicBool::new(false),
            recording: AtomicBool::new(false),
        }
    }
}

/// Shared session data for the audio thread.
/// The UI thread updates this; the audio thread reads it via `try_lock`.
pub struct SharedSessionData {
    pub tracks: Vec<Track>,
    pub audio_pool: Arc<AudioPool>,
    pub sample_rate: u32,
}

/// Audio engine controller — bridges the UI to real-time audio output.
///
/// Opens a cpal output stream and runs a `Timeline` renderer in the audio
/// callback.  Transport state (position, playing, recording) is shared with
/// the audio thread through lock-free atomics.  Session data (tracks and
/// audio pool) is shared through a `Mutex` that the audio thread only
/// `try_lock`s so it never blocks the real-time path.
pub struct AudioEngine {
    pub transport: Arc<SharedTransport>,
    session_data: Arc<Mutex<SharedSessionData>>,
    _output_stream: Option<Box<dyn AudioStream>>,
    _input_stream: Option<Box<dyn AudioStream>>,
    /// Lock-free peak levels: one stereo pair per track slot.
    /// The last slot is the master / mixed output.
    pub meter_levels: SharedMeterLevels,
    /// Accumulates incoming audio from the input callback during recording.
    record_buffer: Arc<Mutex<Vec<f32>>>,
    /// Sample rate used by this engine instance.
    sample_rate: u32,
    /// Recording configuration (sample rate, channels, max duration).
    recording_config: RecordingConfig,
}

impl AudioEngine {
    /// Create a new `AudioEngine` and start the output stream.
    ///
    /// Returns `Err` if the audio backend cannot be initialised (e.g. no
    /// audio device available — common in CI).
    pub fn new(
        session: &Session,
        audio_pool: Arc<AudioPool>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let transport = Arc::new(SharedTransport::new());
        // Pre-allocate enough meter slots for tracks + master.
        // Use a generous capacity so resizes are rare.
        let meter_levels = shared_meter_levels((session.tracks.len() + 1).max(64));

        let session_data = Arc::new(Mutex::new(SharedSessionData {
            tracks: session.tracks.clone(),
            audio_pool,
            sample_rate: session.sample_rate,
        }));

        let backend = CpalBackend::new();
        let format = AudioFormat::new(session.sample_rate, 2, session.buffer_size);

        let transport_cb = Arc::clone(&transport);
        let session_data_cb = Arc::clone(&session_data);
        let meter_levels_cb: SharedMeterLevels = Arc::clone(&meter_levels);

        let callback = Self::build_callback(transport_cb, session_data_cb, meter_levels_cb);

        let stream = backend.open_output_stream(None, format, callback)?;
        stream.start()?;

        Ok(Self {
            transport,
            session_data,
            _output_stream: Some(stream),
            _input_stream: None,
            meter_levels,
            record_buffer: Arc::new(Mutex::new(Vec::new())),
            sample_rate: session.sample_rate,
            recording_config: RecordingConfig::default(),
        })
    }

    /// Build the audio output callback closure.
    fn build_callback(
        transport: Arc<SharedTransport>,
        session_data: Arc<Mutex<SharedSessionData>>,
        meter_levels: SharedMeterLevels,
    ) -> AudioCallback {
        // Per-callback scratch state — lives inside the closure, no locking needed.
        let mut timeline: Option<Timeline> = None;
        let mut render_buf: Option<AudioBuffer> = None;

        Box::new(move |output: &mut [f32]| {
            let is_playing = transport.playing.load(Ordering::Acquire);

            if !is_playing {
                output.fill(0.0);
                return;
            }

            let position = transport.position.load(Ordering::Acquire);
            let channels: u16 = 2;
            let frames = (output.len() / channels as usize) as u32;

            // Try to lock session data — if the UI thread holds the lock, output silence
            // rather than blocking the audio thread.
            let Ok(data) = session_data.try_lock() else {
                output.fill(0.0);
                return;
            };

            // Lazily initialise (or resize) scratch buffers.
            let tl = timeline.get_or_insert_with(|| Timeline::new(channels, frames));
            let buf = render_buf.get_or_insert_with(|| AudioBuffer::new(channels, frames));

            // Recreate if frame count changed (cpal may call us with varying sizes).
            if buf.frames() != frames {
                *tl = Timeline::new(channels, frames);
                *buf = AudioBuffer::new(channels, frames);
            }

            // Build a temporary transport snapshot for the renderer.
            let mut render_transport = Transport::new(data.sample_rate);
            render_transport.position = position;

            tl.render(&data.tracks, &render_transport, &data.audio_pool, buf);

            // Copy rendered audio into the output slice.
            let interleaved = buf.as_interleaved();
            let copy_len = output.len().min(interleaved.len());
            output[..copy_len].copy_from_slice(&interleaved[..copy_len]);
            if copy_len < output.len() {
                output[copy_len..].fill(0.0);
            }

            // Compute master peak levels from the mixed output (lock-free).
            {
                let mut peak_l: f32 = 0.0;
                let mut peak_r: f32 = 0.0;
                for i in (0..copy_len).step_by(channels as usize) {
                    peak_l = peak_l.max(output[i].abs());
                    if i + 1 < copy_len {
                        peak_r = peak_r.max(output[i + 1].abs());
                    }
                }
                // Write to last active slot (master).
                let active = meter_levels.len();
                if active > 0 {
                    meter_levels.store(active - 1, peak_l, peak_r);
                }
            }

            // Advance playback position.
            let new_pos = position + frames as u64;
            transport.position.store(new_pos, Ordering::Release);
        })
    }

    // -- Transport controls ---------------------------------------------------

    /// Start playback from the current position.
    pub fn play(&self) {
        self.transport.playing.store(true, Ordering::Release);
    }

    /// Stop playback and reset position to zero.
    pub fn stop(&self) {
        self.transport.position.store(0, Ordering::Release);
        self.transport.playing.store(false, Ordering::Release);
    }

    /// Pause playback (keeps the current position).
    pub fn pause(&self) {
        self.transport.playing.store(false, Ordering::Release);
    }

    /// Seek to a frame position.
    pub fn seek(&self, position: u64) {
        self.transport.position.store(position, Ordering::Release);
    }

    /// Current playback position in frames.
    pub fn position(&self) -> u64 {
        self.transport.position.load(Ordering::Acquire)
    }

    /// Whether playback is currently active.
    pub fn is_playing(&self) -> bool {
        self.transport.playing.load(Ordering::Acquire)
    }

    // -- Session data updates -------------------------------------------------

    /// Push updated track/pool data to the audio thread.
    ///
    /// Call this after any change to tracks or the audio pool.
    pub fn update_session(&self, session: &Session, audio_pool: Arc<AudioPool>) {
        let new_track_count = session.tracks.len();
        match self.session_data.lock() {
            Ok(mut data) => {
                data.tracks = session.tracks.clone();
                data.audio_pool = audio_pool;
                data.sample_rate = session.sample_rate;
            }
            Err(e) => {
                eprintln!(
                    "shruti-engine: session data mutex poisoned in update_session, recovering: {e}"
                );
                let mut data = e.into_inner();
                data.tracks = session.tracks.clone();
                data.audio_pool = audio_pool;
                data.sample_rate = session.sample_rate;
            }
        }
        // Update active meter slot count (lock-free).
        self.meter_levels.set_active(new_track_count + 1);
    }

    // -- Metering -------------------------------------------------------------

    /// Read current peak meter levels (lock-free).
    ///
    /// Returns a `Vec` with one `[left, right]` pair per track; the last
    /// entry is the master output.
    pub fn read_meters(&self) -> Vec<[f32; 2]> {
        self.meter_levels.read_all()
    }

    // -- Recording ------------------------------------------------------------

    /// The sample rate this engine was initialised with.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Set the recording configuration.
    ///
    /// Must be called before `start_recording()`. Values are validated
    /// (clamped to supported ranges) on assignment.
    pub fn set_recording_config(&mut self, config: RecordingConfig) {
        self.recording_config = config.validated();
    }

    /// Get the current recording configuration.
    pub fn recording_config(&self) -> &RecordingConfig {
        &self.recording_config
    }

    /// The recording sample rate (may differ from playback sample rate).
    pub fn recording_sample_rate(&self) -> u32 {
        self.recording_config.sample_rate
    }

    /// The number of recording channels.
    pub fn recording_channels(&self) -> u16 {
        self.recording_config.channels
    }

    /// Start capturing audio from the configured input device.
    ///
    /// Uses the recording config for sample rate, channel count, and buffer
    /// limits. Clears the internal record buffer and opens an input stream
    /// whose callback appends incoming samples.
    pub fn start_recording(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.transport.recording.store(true, Ordering::Release);

        let max_samples = self.recording_config.max_buffer_samples();
        let rec_channels = self.recording_config.channels;
        let rec_sample_rate = self.recording_config.sample_rate;
        let rec_buffer_size = self.recording_config.buffer_size;

        // Pre-allocate buffer for ~10 seconds to reduce early reallocations
        let pre_alloc = (rec_sample_rate as usize * rec_channels as usize * 10).min(max_samples);
        match self.record_buffer.lock() {
            Ok(mut buf) => {
                buf.clear();
                buf.reserve(pre_alloc);
            }
            Err(e) => {
                eprintln!("shruti-engine: record buffer mutex poisoned, recovering: {e}");
                let mut buf = e.into_inner();
                buf.clear();
                buf.reserve(pre_alloc);
            }
        }

        let record_buf = Arc::clone(&self.record_buffer);
        let backend = CpalBackend::new();
        let format = AudioFormat::new(rec_sample_rate, rec_channels, rec_buffer_size);
        let device_name = self.recording_config.input_device.clone();

        let callback: shruti_engine::backend::InputCallback = Box::new(move |data: &[f32]| {
            if let Ok(mut buf) = record_buf.try_lock() {
                let remaining = max_samples.saturating_sub(buf.len());
                let to_copy = data.len().min(remaining);
                if to_copy > 0 {
                    buf.extend_from_slice(&data[..to_copy]);
                }
            }
        });

        let stream = backend.open_input_stream(device_name.as_deref(), format, callback)?;
        stream.start()?;
        self._input_stream = Some(stream);

        Ok(())
    }

    /// Stop recording and return the captured audio samples.
    ///
    /// Drops the input stream and drains the record buffer. Returns `None`
    /// if no samples were captured. The caller should use
    /// `recording_channels()` and `recording_sample_rate()` to interpret
    /// the returned interleaved sample data.
    pub fn stop_recording(&mut self) -> Option<Vec<f32>> {
        self.transport.recording.store(false, Ordering::Release);
        self._input_stream = None; // Drop the stream, stopping capture

        let mut buf = match self.record_buffer.lock() {
            Ok(buf) => buf,
            Err(e) => {
                eprintln!(
                    "shruti-engine: record buffer mutex poisoned in stop_recording, recovering: {e}"
                );
                e.into_inner()
            }
        };
        if buf.is_empty() {
            return None;
        }
        Some(std::mem::take(&mut *buf))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_transport_recording_flag() {
        let transport = SharedTransport::new();
        assert!(!transport.recording.load(Ordering::Relaxed));
        transport.recording.store(true, Ordering::Relaxed);
        assert!(transport.recording.load(Ordering::Relaxed));
    }
}
