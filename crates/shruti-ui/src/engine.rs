use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use shruti_dsp::{AudioBuffer, AudioFormat};

/// Type alias for the audio output callback closure.
type AudioCallback = Box<dyn FnMut(&mut [f32]) + Send + 'static>;
use shruti_engine::AudioStream;
use shruti_engine::backend::{AudioHost, CpalBackend};
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
    /// Peak levels: Vec of (peak_left, peak_right) per track slot.
    /// The last slot is the master / mixed output.
    pub meter_levels: Arc<Mutex<Vec<[f32; 2]>>>,
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
        let meter_levels: Arc<Mutex<Vec<[f32; 2]>>> =
            Arc::new(Mutex::new(vec![[0.0; 2]; session.tracks.len() + 1]));

        let session_data = Arc::new(Mutex::new(SharedSessionData {
            tracks: session.tracks.clone(),
            audio_pool,
            sample_rate: session.sample_rate,
        }));

        let backend = CpalBackend::new();
        let format = AudioFormat::new(session.sample_rate, 2, session.buffer_size);

        let transport_cb = Arc::clone(&transport);
        let session_data_cb = Arc::clone(&session_data);
        let meter_levels_cb = Arc::clone(&meter_levels);

        let callback = Self::build_callback(transport_cb, session_data_cb, meter_levels_cb);

        let stream = backend.open_output_stream(None, format, callback)?;
        stream.start()?;

        Ok(Self {
            transport,
            session_data,
            _output_stream: Some(stream),
            meter_levels,
        })
    }

    /// Build the audio output callback closure.
    fn build_callback(
        transport: Arc<SharedTransport>,
        session_data: Arc<Mutex<SharedSessionData>>,
        meter_levels: Arc<Mutex<Vec<[f32; 2]>>>,
    ) -> AudioCallback {
        // Per-callback scratch state — lives inside the closure, no locking needed.
        let mut timeline: Option<Timeline> = None;
        let mut render_buf: Option<AudioBuffer> = None;

        Box::new(move |output: &mut [f32]| {
            let is_playing = transport.playing.load(Ordering::Relaxed);

            if !is_playing {
                output.fill(0.0);
                return;
            }

            let position = transport.position.load(Ordering::Relaxed);
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

            // Compute master peak levels from the mixed output.
            if let Ok(mut levels) = meter_levels.try_lock() {
                let mut peak_l: f32 = 0.0;
                let mut peak_r: f32 = 0.0;
                for i in (0..copy_len).step_by(channels as usize) {
                    peak_l = peak_l.max(output[i].abs());
                    if i + 1 < copy_len {
                        peak_r = peak_r.max(output[i + 1].abs());
                    }
                }
                if let Some(last) = levels.last_mut() {
                    *last = [peak_l, peak_r];
                }
            }

            // Advance playback position.
            let new_pos = position + frames as u64;
            transport.position.store(new_pos, Ordering::Relaxed);
        })
    }

    // -- Transport controls ---------------------------------------------------

    /// Start playback from the current position.
    pub fn play(&self) {
        self.transport.playing.store(true, Ordering::Relaxed);
    }

    /// Stop playback and reset position to zero.
    pub fn stop(&self) {
        self.transport.playing.store(false, Ordering::Relaxed);
        self.transport.position.store(0, Ordering::Relaxed);
    }

    /// Pause playback (keeps the current position).
    pub fn pause(&self) {
        self.transport.playing.store(false, Ordering::Relaxed);
    }

    /// Seek to a frame position.
    pub fn seek(&self, position: u64) {
        self.transport.position.store(position, Ordering::Relaxed);
    }

    /// Current playback position in frames.
    pub fn position(&self) -> u64 {
        self.transport.position.load(Ordering::Relaxed)
    }

    /// Whether playback is currently active.
    pub fn is_playing(&self) -> bool {
        self.transport.playing.load(Ordering::Relaxed)
    }

    // -- Session data updates -------------------------------------------------

    /// Push updated track/pool data to the audio thread.
    ///
    /// Call this after any change to tracks or the audio pool.
    pub fn update_session(&self, session: &Session, audio_pool: Arc<AudioPool>) {
        if let Ok(mut data) = self.session_data.lock() {
            data.tracks = session.tracks.clone();
            data.audio_pool = audio_pool;
            data.sample_rate = session.sample_rate;
        }
        if let Ok(mut levels) = self.meter_levels.lock() {
            levels.resize(session.tracks.len() + 1, [0.0; 2]);
        }
    }

    // -- Metering -------------------------------------------------------------

    /// Read current peak meter levels.
    ///
    /// Returns a `Vec` with one `[left, right]` pair per track; the last
    /// entry is the master output.
    pub fn read_meters(&self) -> Vec<[f32; 2]> {
        self.meter_levels
            .lock()
            .map(|l| l.clone())
            .unwrap_or_default()
    }
}
