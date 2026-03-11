use midir::{MidiInput, MidiOutput};

/// Information about a MIDI port.
#[derive(Debug, Clone)]
pub struct MidiPortInfo {
    pub name: String,
    pub is_input: bool,
    pub is_output: bool,
}

/// Enumerate all available MIDI input and output ports.
pub fn enumerate_midi_ports() -> Vec<MidiPortInfo> {
    let mut ports = Vec::new();

    if let Ok(midi_in) = MidiInput::new("shruti-scan") {
        for port in midi_in.ports() {
            if let Ok(name) = midi_in.port_name(&port) {
                ports.push(MidiPortInfo {
                    name,
                    is_input: true,
                    is_output: false,
                });
            }
        }
    }

    if let Ok(midi_out) = MidiOutput::new("shruti-scan") {
        for port in midi_out.ports() {
            if let Ok(name) = midi_out.port_name(&port) {
                ports.push(MidiPortInfo {
                    name,
                    is_input: false,
                    is_output: true,
                });
            }
        }
    }

    ports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerate_midi_ports_does_not_panic() {
        // Just ensures we can call it without panicking (no devices in CI)
        let ports = enumerate_midi_ports();
        // ports may be empty in CI, that's fine
        for port in &ports {
            assert!(!port.name.is_empty());
            assert!(port.is_input || port.is_output);
        }
    }

    #[test]
    fn test_midi_port_info_fields() {
        let port = MidiPortInfo {
            name: "Test MIDI Port".to_string(),
            is_input: true,
            is_output: false,
        };
        assert_eq!(port.name, "Test MIDI Port");
        assert!(port.is_input);
        assert!(!port.is_output);
    }

    #[test]
    fn test_midi_port_info_clone() {
        let port = MidiPortInfo {
            name: "Cloned Port".to_string(),
            is_input: false,
            is_output: true,
        };
        let cloned = port.clone();
        assert_eq!(cloned.name, port.name);
        assert_eq!(cloned.is_input, port.is_input);
        assert_eq!(cloned.is_output, port.is_output);
    }

    #[test]
    fn test_midi_port_info_debug() {
        let port = MidiPortInfo {
            name: "Debug Port".to_string(),
            is_input: true,
            is_output: true,
        };
        let debug_str = format!("{:?}", port);
        assert!(debug_str.contains("Debug Port"));
        assert!(debug_str.contains("is_input: true"));
        assert!(debug_str.contains("is_output: true"));
    }

    #[test]
    fn test_enumerate_returns_correct_flags() {
        let ports = enumerate_midi_ports();
        for port in &ports {
            // Each port from enumerate should be exclusively input or output
            assert!(port.is_input ^ port.is_output);
        }
    }
}
