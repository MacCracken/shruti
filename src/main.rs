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
        None => {
            println!("shruti v{}", env!("CARGO_PKG_VERSION"));
            println!("Run `shruti --help` for usage.");
        }
    }
}
