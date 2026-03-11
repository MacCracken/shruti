use egui::Key;

use super::actions::Action;
use super::shortcuts::{Shortcut, ShortcutRegistry};

/// Load the default DAW keybindings (Logic/Reaper hybrid style).
pub fn default_keymap() -> ShortcutRegistry {
    let mut reg = ShortcutRegistry::new();

    // Transport
    reg.bind(Shortcut::key(Key::Space), Action::Play);
    reg.bind(Shortcut::key(Key::Enter), Action::Stop);
    reg.bind(Shortcut::key(Key::R), Action::Record);
    reg.bind(Shortcut::key(Key::L), Action::ToggleLoop);
    reg.bind(Shortcut::key(Key::Home), Action::GoToStart);
    reg.bind(Shortcut::key(Key::End), Action::GoToEnd);

    // Editing
    reg.bind(Shortcut::ctrl(Key::Z), Action::Undo);
    reg.bind(Shortcut::ctrl_shift(Key::Z), Action::Redo);
    reg.bind(Shortcut::ctrl(Key::X), Action::Cut);
    reg.bind(Shortcut::ctrl(Key::C), Action::Copy);
    reg.bind(Shortcut::ctrl(Key::V), Action::Paste);
    reg.bind(Shortcut::key(Key::Delete), Action::Delete);
    reg.bind(Shortcut::key(Key::Backspace), Action::Delete);
    reg.bind(Shortcut::ctrl(Key::A), Action::SelectAll);
    reg.bind(Shortcut::key(Key::S), Action::SplitAtPlayhead);
    reg.bind(Shortcut::ctrl(Key::D), Action::Duplicate);

    // View
    reg.bind(Shortcut::key(Key::F1), Action::ToggleArrangement);
    reg.bind(Shortcut::key(Key::F2), Action::ToggleMixer);
    reg.bind(Shortcut::key(Key::F3), Action::ToggleBrowser);
    reg.bind(Shortcut::ctrl(Key::ArrowUp), Action::ZoomIn);
    reg.bind(Shortcut::ctrl(Key::ArrowDown), Action::ZoomOut);
    reg.bind(Shortcut::ctrl(Key::F), Action::ZoomToFit);

    // Tracks
    reg.bind(Shortcut::ctrl(Key::T), Action::NewAudioTrack);
    reg.bind(Shortcut::key(Key::M), Action::ToggleMute);

    // File
    reg.bind(Shortcut::ctrl(Key::N), Action::NewSession);
    reg.bind(Shortcut::ctrl(Key::O), Action::OpenSession);
    reg.bind(Shortcut::ctrl(Key::S), Action::SaveSession);

    reg
}
