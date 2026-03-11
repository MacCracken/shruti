//! GPU-accelerated DAW interface for Shruti.
//!
//! Built on egui + eframe (wgpu + winit), providing a cross-platform
//! immediate-mode UI with custom-painted DAW widgets.

pub mod app;
pub mod engine;
pub mod input;
pub mod state;
pub mod theme;
pub mod views;
pub mod widgets;

pub use app::ShrutiApp;
pub use state::UiState;
pub use theme::Theme;

/// Launch the Shruti GUI.
pub fn run(session: shruti_session::Session) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Shruti")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Shruti",
        options,
        Box::new(|_cc| {
            let state = UiState::new(session);
            Ok(Box::new(ShrutiApp::new(state)))
        }),
    )
}

/// Launch the Shruti GUI with a custom theme.
pub fn run_with_theme(session: shruti_session::Session, theme: Theme) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Shruti")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Shruti",
        options,
        Box::new(move |_cc| {
            let state = UiState::new(session);
            Ok(Box::new(ShrutiApp::new(state).with_theme(theme)))
        }),
    )
}
