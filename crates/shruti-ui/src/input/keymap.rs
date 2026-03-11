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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_keymap_has_transport_bindings() {
        let km = default_keymap();
        assert_eq!(km.lookup(&Shortcut::key(Key::Space)), Some(Action::Play));
        assert_eq!(km.lookup(&Shortcut::key(Key::Enter)), Some(Action::Stop));
        assert_eq!(km.lookup(&Shortcut::key(Key::R)), Some(Action::Record));
        assert_eq!(km.lookup(&Shortcut::key(Key::L)), Some(Action::ToggleLoop));
        assert_eq!(
            km.lookup(&Shortcut::key(Key::Home)),
            Some(Action::GoToStart)
        );
        assert_eq!(km.lookup(&Shortcut::key(Key::End)), Some(Action::GoToEnd));
    }

    #[test]
    fn default_keymap_has_editing_bindings() {
        let km = default_keymap();
        assert_eq!(km.lookup(&Shortcut::ctrl(Key::Z)), Some(Action::Undo));
        assert_eq!(km.lookup(&Shortcut::ctrl_shift(Key::Z)), Some(Action::Redo));
        assert_eq!(km.lookup(&Shortcut::ctrl(Key::X)), Some(Action::Cut));
        assert_eq!(km.lookup(&Shortcut::ctrl(Key::C)), Some(Action::Copy));
        assert_eq!(km.lookup(&Shortcut::ctrl(Key::V)), Some(Action::Paste));
        assert_eq!(km.lookup(&Shortcut::key(Key::Delete)), Some(Action::Delete));
        assert_eq!(
            km.lookup(&Shortcut::key(Key::Backspace)),
            Some(Action::Delete)
        );
        assert_eq!(km.lookup(&Shortcut::ctrl(Key::A)), Some(Action::SelectAll));
        assert_eq!(
            km.lookup(&Shortcut::key(Key::S)),
            Some(Action::SplitAtPlayhead)
        );
        assert_eq!(km.lookup(&Shortcut::ctrl(Key::D)), Some(Action::Duplicate));
    }

    #[test]
    fn default_keymap_has_view_bindings() {
        let km = default_keymap();
        assert_eq!(
            km.lookup(&Shortcut::key(Key::F1)),
            Some(Action::ToggleArrangement)
        );
        assert_eq!(
            km.lookup(&Shortcut::key(Key::F2)),
            Some(Action::ToggleMixer)
        );
        assert_eq!(
            km.lookup(&Shortcut::key(Key::F3)),
            Some(Action::ToggleBrowser)
        );
        assert_eq!(
            km.lookup(&Shortcut::ctrl(Key::ArrowUp)),
            Some(Action::ZoomIn)
        );
        assert_eq!(
            km.lookup(&Shortcut::ctrl(Key::ArrowDown)),
            Some(Action::ZoomOut)
        );
        assert_eq!(km.lookup(&Shortcut::ctrl(Key::F)), Some(Action::ZoomToFit));
    }

    #[test]
    fn default_keymap_has_file_bindings() {
        let km = default_keymap();
        assert_eq!(km.lookup(&Shortcut::ctrl(Key::N)), Some(Action::NewSession));
        assert_eq!(
            km.lookup(&Shortcut::ctrl(Key::O)),
            Some(Action::OpenSession)
        );
        assert_eq!(
            km.lookup(&Shortcut::ctrl(Key::S)),
            Some(Action::SaveSession)
        );
    }

    #[test]
    fn default_keymap_has_track_bindings() {
        let km = default_keymap();
        assert_eq!(
            km.lookup(&Shortcut::ctrl(Key::T)),
            Some(Action::NewAudioTrack)
        );
        assert_eq!(km.lookup(&Shortcut::key(Key::M)), Some(Action::ToggleMute));
    }

    #[test]
    fn unbound_key_returns_none() {
        let km = default_keymap();
        assert_eq!(km.lookup(&Shortcut::key(Key::Q)), None);
        assert_eq!(km.lookup(&Shortcut::ctrl(Key::Q)), None);
    }
}
