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
///
/// All fields use `Acquire`/`Release` ordering for correct cross-thread
/// visibility. The audio thread writes `position`; the UI thread writes
/// transport controls (`playing`, `recording`, `seek_position`) and loop
/// settings.
pub struct SharedTransport {
    /// Current playback position in frames (written by audio thread).
    pub position: AtomicU64,
    /// Whether playback is active.
    pub playing: AtomicBool,
    /// Whether recording is active.
    pub recording: AtomicBool,
    /// Seek request: when non-u64::MAX, the audio thread should jump to this
    /// position and reset the flag. This avoids the UI writing `position`
    /// directly while the audio thread is also writing it.
    seek_request: AtomicU64,
    /// Loop enabled flag.
    pub loop_enabled: AtomicBool,
    /// Loop start position in frames.
    pub loop_start: AtomicU64,
    /// Loop end position in frames.
    pub loop_end: AtomicU64,
}

/// Sentinel value meaning "no seek pending".
const NO_SEEK: u64 = u64::MAX;

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
            seek_request: AtomicU64::new(NO_SEEK),
            loop_enabled: AtomicBool::new(false),
            loop_start: AtomicU64::new(0),
            loop_end: AtomicU64::new(0),
        }
    }

    /// Request a seek. The audio thread will pick this up and update position.
    pub fn request_seek(&self, position: u64) {
        self.seek_request.store(position, Ordering::Release);
    }

    /// Update loop settings from the UI transport state.
    pub fn sync_loop(&self, enabled: bool, start: u64, end: u64) {
        self.loop_enabled.store(enabled, Ordering::Release);
        self.loop_start.store(start, Ordering::Release);
        self.loop_end.store(end, Ordering::Release);
    }
}

/// Shared session data for the audio thread.
pub struct SharedSessionData {
    pub tracks: Vec<Track>,
    pub audio_pool: Arc<AudioPool>,
    pub sample_rate: u32,
}

/// Audio engine controller — bridges the UI to real-time audio output.
///
/// Opens a cpal output stream and runs a `Timeline` renderer in the audio
/// callback.  Transport state (position, playing, recording, loop) is shared
/// with the audio thread through lock-free atomics.
///
/// Session data (tracks and audio pool) uses a double-buffer pattern: the UI
/// thread places updates into a `pending_session` slot, and the audio thread
/// picks them up on the next callback cycle.  On contention, the audio thread
/// continues rendering with the previous data instead of outputting silence.
pub struct AudioEngine {
    pub transport: Arc<SharedTransport>,
    /// Pending session data slot — UI writes here, audio thread picks up.
    pending_session: Arc<Mutex<Option<SharedSessionData>>>,
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
        // Sync loop state from session transport.
        transport.sync_loop(
            session.transport.loop_enabled,
            session.transport.loop_start.0,
            session.transport.loop_end.0,
        );

        // Pre-allocate enough meter slots for tracks + master.
        let meter_levels = shared_meter_levels((session.tracks.len() + 1).max(64));

        // Place initial session data as pending so the first callback picks it up.
        let initial_data = SharedSessionData {
            tracks: session.tracks.clone(),
            audio_pool,
            sample_rate: session.sample_rate,
        };
        let pending_session = Arc::new(Mutex::new(Some(initial_data)));

        let backend = CpalBackend::new();
        let format = AudioFormat::new(session.sample_rate, 2, session.buffer_size);

        let transport_cb = Arc::clone(&transport);
        let pending_cb = Arc::clone(&pending_session);
        let meter_levels_cb: SharedMeterLevels = Arc::clone(&meter_levels);

        let callback = Self::build_callback(transport_cb, pending_cb, meter_levels_cb);

        let stream = backend.open_output_stream(None, format, callback)?;
        stream.start()?;

        Ok(Self {
            transport,
            pending_session,
            _output_stream: Some(stream),
            _input_stream: None,
            meter_levels,
            record_buffer: Arc::new(Mutex::new(Vec::new())),
            sample_rate: session.sample_rate,
            recording_config: RecordingConfig::default(),
        })
    }

    /// Build the audio output callback closure.
    ///
    /// Uses double-buffered session data: the closure owns a local copy of the
    /// session data and checks the `pending` slot on each cycle for updates.
    /// On contention, the previous data is used (no silence).
    fn build_callback(
        transport: Arc<SharedTransport>,
        pending: Arc<Mutex<Option<SharedSessionData>>>,
        meter_levels: SharedMeterLevels,
    ) -> AudioCallback {
        // Per-callback scratch state — lives inside the closure, no locking needed.
        let mut timeline: Option<Timeline> = None;
        let mut render_buf: Option<AudioBuffer> = None;
        // Local session data owned by the audio thread — never locked.
        let mut local_data: Option<SharedSessionData> = None;

        Box::new(move |output: &mut [f32]| {
            // Check for a seek request first.
            let seek = transport.seek_request.swap(NO_SEEK, Ordering::AcqRel);
            if seek != NO_SEEK {
                transport.position.store(seek, Ordering::Release);
            }

            let is_playing = transport.playing.load(Ordering::Acquire);

            if !is_playing {
                output.fill(0.0);
                return;
            }

            let position = transport.position.load(Ordering::Acquire);
            let channels: u16 = 2;
            let frames = (output.len() / channels as usize) as u32;

            // Try to pick up pending session data. On contention, keep the
            // previous local copy (no silence).
            match pending.try_lock() {
                Ok(mut guard) => {
                    if let Some(new_data) = guard.take() {
                        local_data = Some(new_data);
                    }
                }
                Err(std::sync::TryLockError::WouldBlock) => {
                    // Contention — continue with existing local_data.
                }
                Err(std::sync::TryLockError::Poisoned(e)) => {
                    eprintln!("shruti-engine: pending session mutex poisoned, recovering: {e}");
                    let mut guard = e.into_inner();
                    if let Some(new_data) = guard.take() {
                        local_data = Some(new_data);
                    }
                }
            }

            let data = match local_data.as_ref() {
                Some(d) => d,
                None => {
                    output.fill(0.0);
                    return;
                }
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
            render_transport.position = shruti_session::FramePos(position);

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

            // Advance playback position with loop handling.
            let loop_enabled = transport.loop_enabled.load(Ordering::Acquire);
            let new_pos = if loop_enabled {
                let loop_start = transport.loop_start.load(Ordering::Acquire);
                let loop_end = transport.loop_end.load(Ordering::Acquire);
                if loop_end > loop_start {
                    let end = position + frames as u64;
                    if end >= loop_end {
                        let loop_length = loop_end - loop_start;
                        let overshoot = end - loop_end;
                        loop_start + (overshoot % loop_length)
                    } else {
                        end
                    }
                } else {
                    position + frames as u64
                }
            } else {
                position + frames as u64
            };
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
        self.transport.playing.store(false, Ordering::Release);
        self.transport.request_seek(0);
    }

    /// Pause playback (keeps the current position).
    pub fn pause(&self) {
        self.transport.playing.store(false, Ordering::Release);
    }

    /// Seek to a frame position.
    pub fn seek(&self, position: u64) {
        self.transport.request_seek(position);
    }

    /// Current playback position in frames.
    pub fn position(&self) -> u64 {
        self.transport.position.load(Ordering::Acquire)
    }

    /// Whether playback is currently active.
    pub fn is_playing(&self) -> bool {
        self.transport.playing.load(Ordering::Acquire)
    }

    /// Sync loop settings from the UI transport to the audio thread.
    pub fn sync_transport(&self, transport: &Transport) {
        self.transport.sync_loop(
            transport.loop_enabled,
            transport.loop_start.0,
            transport.loop_end.0,
        );
    }

    // -- Session data updates -------------------------------------------------

    /// Push updated track/pool data to the audio thread.
    ///
    /// Places the new data in a pending slot that the audio thread will pick
    /// up on its next callback cycle.  If the audio thread is busy, the update
    /// waits briefly (blocking the UI thread, not the audio thread).
    pub fn update_session(&self, session: &Session, audio_pool: Arc<AudioPool>) {
        let new_track_count = session.tracks.len();
        let new_data = SharedSessionData {
            tracks: session.tracks.clone(),
            audio_pool,
            sample_rate: session.sample_rate,
        };
        match self.pending_session.lock() {
            Ok(mut slot) => {
                *slot = Some(new_data);
            }
            Err(e) => {
                eprintln!(
                    "shruti-engine: pending session mutex poisoned in update_session, recovering: {e}"
                );
                let mut slot = e.into_inner();
                *slot = Some(new_data);
            }
        }
        // Update active meter slot count (lock-free).
        self.meter_levels.set_active(new_track_count + 1);
        // Sync loop settings.
        self.sync_transport(&session.transport);
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

    #[test]
    fn shared_transport_seek_request() {
        let transport = SharedTransport::new();
        assert_eq!(transport.seek_request.load(Ordering::Relaxed), NO_SEEK);
        transport.request_seek(48000);
        assert_eq!(transport.seek_request.load(Ordering::Relaxed), 48000);
        // Consuming the seek request (as the audio thread would)
        let seek = transport.seek_request.swap(NO_SEEK, Ordering::AcqRel);
        assert_eq!(seek, 48000);
        assert_eq!(transport.seek_request.load(Ordering::Relaxed), NO_SEEK);
    }

    #[test]
    fn shared_transport_loop_sync() {
        let transport = SharedTransport::new();
        assert!(!transport.loop_enabled.load(Ordering::Relaxed));
        transport.sync_loop(true, 1000, 5000);
        assert!(transport.loop_enabled.load(Ordering::Relaxed));
        assert_eq!(transport.loop_start.load(Ordering::Relaxed), 1000);
        assert_eq!(transport.loop_end.load(Ordering::Relaxed), 5000);
    }

    #[test]
    fn shared_transport_default() {
        let transport = SharedTransport::default();
        assert_eq!(transport.position.load(Ordering::Relaxed), 0);
        assert!(!transport.playing.load(Ordering::Relaxed));
        assert!(!transport.recording.load(Ordering::Relaxed));
        assert!(!transport.loop_enabled.load(Ordering::Relaxed));
    }
}
