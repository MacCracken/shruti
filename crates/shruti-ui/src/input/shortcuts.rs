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
