use clap::Parser;

#[derive(Parser)]
#[command(
    name = "shruti",
    version,
    about = "Shruti — A Rust-native Digital Audio Workstation"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Show version info
    Version,
    /// List audio devices
    Devices,
    /// Launch the GUI (default)
    Gui {
        /// Session file to open
        #[arg(short, long)]
        session: Option<String>,
        /// Theme file to load (JSON)
        #[arg(short, long)]
        theme: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Version) => {
            println!("shruti v{}", env!("CARGO_PKG_VERSION"));
        }
        Some(Commands::Devices) => {
            use shruti_engine::backend::{AudioHost, CpalBackend};
            let backend = CpalBackend::new();

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
        }
        Some(Commands::Gui { session: _, theme }) => {
            let session = shruti_session::Session::new("Untitled", 48000, 256);

            let result = if let Some(ref theme_path) = theme {
                match shruti_ui::Theme::load(std::path::Path::new(theme_path)) {
                    Ok(t) => shruti_ui::run_with_theme(session, t),
                    Err(e) => {
                        eprintln!("Warning: failed to load theme: {e}. Using default.");
                        shruti_ui::run(session)
                    }
                }
            } else {
                shruti_ui::run(session)
            };

            if let Err(e) = result {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        None => {
            // Default: launch GUI with new session
            let session = shruti_session::Session::new("Untitled", 48000, 256);
            if let Err(e) = shruti_ui::run(session) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
    }
}
