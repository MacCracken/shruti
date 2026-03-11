use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use clap::Parser;
use shruti_dsp::io::{read_audio_file, write_wav_file};
use shruti_dsp::{AudioBuffer, AudioFormat};
use shruti_engine::backend::{AudioHost, CpalBackend};
use shruti_engine::graph::{FilePlayerNode, Graph, GraphProcessor, NodeId};

#[derive(Parser)]
#[command(name = "shruti-play", about = "Shruti audio playback and recording tool")]
struct Cli {
    /// Audio file to play (WAV or FLAC)
    file: Option<PathBuf>,

    /// Record from default input device to this WAV file
    #[arg(long)]
    record: Option<PathBuf>,

    /// Audio device name (uses default if not specified)
    #[arg(long)]
    device: Option<String>,

    /// Buffer size in frames
    #[arg(long, default_value = "256")]
    buffer_size: u32,

    /// Sample rate
    #[arg(long, default_value = "48000")]
    sample_rate: u32,

    /// List available audio devices
    #[arg(long)]
    list_devices: bool,

    /// Loop playback
    #[arg(long, short)]
    r#loop: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let backend = CpalBackend::new();

    if cli.list_devices {
        println!("Output devices:");
        for dev in backend.output_devices() {
            let marker = if dev.is_default { " (default)" } else { "" };
            println!("  {}{}", dev.name, marker);
        }
        println!("\nInput devices:");
        for dev in backend.input_devices() {
            let marker = if dev.is_default { " (default)" } else { "" };
            println!("  {}{}", dev.name, marker);
        }
        return Ok(());
    }

    if let Some(output_path) = cli.record {
        return record(&backend, cli.device.as_ref(), cli.buffer_size, cli.sample_rate, &output_path);
    }

    let file = cli.file.ok_or("provide a file to play, or use --record <path>")?;
    play(&backend, cli.device.as_ref(), cli.buffer_size, &file, cli.r#loop)
}

fn play(
    backend: &CpalBackend,
    device: Option<&String>,
    buffer_size: u32,
    path: &PathBuf,
    looping: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let (buffer, file_format) = read_audio_file(path)?;
    let frames = buffer.frames();
    let channels = buffer.channels();
    let duration_secs = frames as f64 / file_format.sample_rate as f64;

    println!(
        "Playing: {} ({:.1}s, {}Hz, {}ch)",
        path.display(),
        duration_secs,
        file_format.sample_rate,
        channels,
    );

    let format = AudioFormat::new(file_format.sample_rate, channels, buffer_size);

    let player_id = NodeId::next();
    let mut graph = Graph::new();
    graph.add_node(player_id, Box::new(FilePlayerNode::new(buffer, looping)));
    let plan = graph.compile()?;

    let mut processor = GraphProcessor::new();
    let handle = processor.swap_handle();
    handle.swap(plan);

    let finished = Arc::new(AtomicBool::new(false));
    let finished_cb = Arc::clone(&finished);

    // Ctrl+C handler
    let ctrlc_flag = Arc::clone(&finished);
    ctrlc_handler(ctrlc_flag);

    let stream = backend.open_output_stream(
        device.map(|s| s.as_str()),
        format,
        Box::new(move |data: &mut [f32]| {
            let buf_frames = data.len() as u32 / channels as u32;
            processor.process(data, channels, buf_frames);

            if processor.is_finished() {
                finished_cb.store(true, Ordering::Relaxed);
            }
        }),
    )?;

    stream.start()?;

    while !finished.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    stream.stop()?;
    println!("Done.");
    Ok(())
}

fn record(
    backend: &CpalBackend,
    device: Option<&String>,
    buffer_size: u32,
    sample_rate: u32,
    output_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let channels = 2u16;
    let format = AudioFormat::new(sample_rate, channels, buffer_size);

    println!(
        "Recording to: {} ({}Hz, {}ch) — press Ctrl+C to stop",
        output_path.display(),
        sample_rate,
        channels,
    );

    // Use a ring buffer: RT callback pushes, main thread drains
    let capacity = sample_rate as usize * channels as usize * 10;
    let (mut producer, mut consumer) = rtrb::RingBuffer::new(capacity);

    let stop = Arc::new(AtomicBool::new(false));
    ctrlc_handler(Arc::clone(&stop));

    let stream = backend.open_input_stream(
        device.map(|s| s.as_str()),
        format,
        Box::new(move |data: &[f32]| {
            for &sample in data {
                let _ = producer.push(sample);
            }
        }),
    )?;

    stream.start()?;

    let mut all_samples: Vec<f32> = Vec::new();

    while !stop.load(Ordering::Relaxed) {
        // Drain available samples from the ring buffer
        while let Ok(sample) = consumer.pop() {
            all_samples.push(sample);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    stream.stop()?;

    // Final drain
    while let Ok(sample) = consumer.pop() {
        all_samples.push(sample);
    }

    println!("Recording stopped. Saving...");

    let buffer = AudioBuffer::from_interleaved(all_samples, channels);
    let audio_format = AudioFormat::new(sample_rate, channels, 0);
    write_wav_file(output_path, &buffer, &audio_format)?;

    println!("Saved to: {}", output_path.display());
    Ok(())
}

fn ctrlc_handler(_flag: Arc<AtomicBool>) {
    // TODO: Add signal-hook crate for proper Ctrl+C handling
    // For now, Ctrl+C terminates the process directly
}
