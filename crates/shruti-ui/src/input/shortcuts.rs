use std::collections::HashMap;

use egui::{Key, Modifiers};

use super::actions::Action;

/// A keyboard shortcut: key + modifier combination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl Shortcut {
    pub const fn new(key: Key, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }

    pub const fn key(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::NONE,
        }
    }

    pub const fn ctrl(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::CTRL,
        }
    }

    pub const fn ctrl_shift(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::COMMAND.plus(Modifiers::SHIFT),
        }
    }
}

/// Registry mapping keyboard shortcuts to actions.
pub struct ShortcutRegistry {
    bindings: HashMap<Shortcut, Action>,
}

impl ShortcutRegistry {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    pub fn bind(&mut self, shortcut: Shortcut, action: Action) {
        self.bindings.insert(shortcut, action);
    }

    pub fn lookup(&self, shortcut: &Shortcut) -> Option<Action> {
        self.bindings.get(shortcut).copied()
    }

    /// Check egui input for any matching shortcut and return the action.
    pub fn check_input(&self, ctx: &egui::Context) -> Option<Action> {
        ctx.input(|input| {
            for (shortcut, action) in &self.bindings {
                if input.modifiers == shortcut.modifiers && input.key_pressed(shortcut.key) {
                    return Some(*action);
                }
            }
            None
        })
    }
}

impl Default for ShortcutRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_key_has_no_modifiers() {
        let s = Shortcut::key(Key::Space);
        assert_eq!(s.key, Key::Space);
        assert_eq!(s.modifiers, Modifiers::NONE);
    }

    #[test]
    fn shortcut_ctrl_has_ctrl_modifier() {
        let s = Shortcut::ctrl(Key::Z);
        assert_eq!(s.key, Key::Z);
        assert_eq!(s.modifiers, Modifiers::CTRL);
    }

    #[test]
    fn shortcut_ctrl_shift() {
        let s = Shortcut::ctrl_shift(Key::Z);
        assert_eq!(s.key, Key::Z);
        assert!(s.modifiers.shift);
        assert!(s.modifiers.command || s.modifiers.ctrl);
    }

    #[test]
    fn shortcut_new_with_custom_modifiers() {
        let mods = Modifiers::ALT;
        let s = Shortcut::new(Key::A, mods);
        assert_eq!(s.key, Key::A);
        assert_eq!(s.modifiers, Modifiers::ALT);
    }

    #[test]
    fn shortcut_equality() {
        let a = Shortcut::key(Key::Space);
        let b = Shortcut::key(Key::Space);
        assert_eq!(a, b);

        let c = Shortcut::ctrl(Key::Space);
        assert_ne!(a, c);
    }

    #[test]
    fn registry_new_is_empty() {
        let reg = ShortcutRegistry::new();
        assert_eq!(reg.lookup(&Shortcut::key(Key::Space)), None);
    }

    #[test]
    fn registry_default_is_empty() {
        let reg = ShortcutRegistry::default();
        assert_eq!(reg.lookup(&Shortcut::key(Key::Space)), None);
    }

    #[test]
    fn bind_and_lookup() {
        let mut reg = ShortcutRegistry::new();
        reg.bind(Shortcut::key(Key::Space), Action::Play);
        assert_eq!(reg.lookup(&Shortcut::key(Key::Space)), Some(Action::Play));
    }

    #[test]
    fn lookup_missing_returns_none() {
        let mut reg = ShortcutRegistry::new();
        reg.bind(Shortcut::key(Key::Space), Action::Play);
        assert_eq!(reg.lookup(&Shortcut::ctrl(Key::Space)), None);
    }

    #[test]
    fn bind_overwrites_previous() {
        let mut reg = ShortcutRegistry::new();
        reg.bind(Shortcut::key(Key::Space), Action::Play);
        reg.bind(Shortcut::key(Key::Space), Action::Stop);
        assert_eq!(reg.lookup(&Shortcut::key(Key::Space)), Some(Action::Stop));
    }

    #[test]
    fn multiple_bindings() {
        let mut reg = ShortcutRegistry::new();
        reg.bind(Shortcut::key(Key::Space), Action::Play);
        reg.bind(Shortcut::key(Key::Enter), Action::Stop);
        reg.bind(Shortcut::ctrl(Key::Z), Action::Undo);

        assert_eq!(reg.lookup(&Shortcut::key(Key::Space)), Some(Action::Play));
        assert_eq!(reg.lookup(&Shortcut::key(Key::Enter)), Some(Action::Stop));
        assert_eq!(reg.lookup(&Shortcut::ctrl(Key::Z)), Some(Action::Undo));
    }
}
