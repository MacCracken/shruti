use serde::{Deserialize, Serialize};

/// A parsed voice/text intent for controlling Shruti.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceIntent {
    /// The action category.
    pub action: VoiceAction,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,
    /// Original input text.
    pub original: String,
}

/// Supported voice action types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VoiceAction {
    /// Transport controls: play, stop, pause, record.
    Transport(TransportCommand),
    /// Seek to a position: "go to bar 16", "jump to the chorus".
    Seek(SeekTarget),
    /// Track control: mute, solo, volume, pan.
    TrackControl(TrackCommand),
    /// Mixing: "make it louder", "add reverb".
    Mix(MixCommand),
    /// Tempo: "set tempo to 120", "faster", "slower".
    Tempo(TempoCommand),
    /// Analysis: "analyze the mix", "check the levels".
    Analyze(AnalyzeCommand),
    /// Unknown / could not parse.
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransportCommand {
    Play,
    Stop,
    Pause,
    Record,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SeekTarget {
    Bar(u64),
    Beginning,
    End,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrackCommand {
    Mute(String),
    Unmute(String),
    Solo(String),
    Unsolo(String),
    Volume {
        track: String,
        direction: Direction,
    },
    Pan {
        track: String,
        direction: PanDirection,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Set(f32),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PanDirection {
    Left,
    Right,
    Center,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MixCommand {
    AutoMix,
    AddEffect { track: String, effect: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TempoCommand {
    Set(f64),
    Faster,
    Slower,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnalyzeCommand {
    Spectrum(String),
    Dynamics(String),
    FullMix,
}

/// Parse a natural language input into a VoiceIntent.
pub fn parse_voice_input(input: &str) -> VoiceIntent {
    let lower = input.trim().to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    let (action, confidence) = parse_action(&lower, &words);

    VoiceIntent {
        action,
        confidence,
        original: input.to_string(),
    }
}

fn parse_action(lower: &str, words: &[&str]) -> (VoiceAction, f32) {
    // Seek commands (check before transport — "play from bar 8" is a seek, not play)
    if (lower.contains("go to bar")
        || lower.contains("jump to bar")
        || lower.contains("skip to bar"))
        && let Some(bar) = extract_number(words)
    {
        return (VoiceAction::Seek(SeekTarget::Bar(bar as u64)), 0.90);
    }
    if (lower.contains("from bar") || lower.contains("play from bar"))
        && let Some(bar) = extract_number(words)
    {
        return (VoiceAction::Seek(SeekTarget::Bar(bar as u64)), 0.85);
    }

    // Transport commands (high confidence — unambiguous)
    if matches_any(
        lower,
        &["play", "start playing", "start playback", "hit play"],
    ) {
        return (VoiceAction::Transport(TransportCommand::Play), 0.95);
    }
    if matches_any(
        lower,
        &["stop", "stop playing", "stop playback", "hit stop"],
    ) {
        return (VoiceAction::Transport(TransportCommand::Stop), 0.95);
    }
    if matches_any(lower, &["pause", "hold", "freeze"]) {
        return (VoiceAction::Transport(TransportCommand::Pause), 0.90);
    }
    if matches_any(
        lower,
        &["record", "start recording", "arm and record", "hit record"],
    ) {
        return (VoiceAction::Transport(TransportCommand::Record), 0.95);
    }
    if matches_any(
        lower,
        &[
            "go to the beginning",
            "go to start",
            "rewind",
            "back to start",
        ],
    ) {
        return (VoiceAction::Seek(SeekTarget::Beginning), 0.90);
    }
    if matches_any(lower, &["go to the end", "jump to end", "skip to end"]) {
        return (VoiceAction::Seek(SeekTarget::End), 0.90);
    }

    // Mute/unmute
    if lower.contains("mute") && !lower.contains("unmute") {
        let track = extract_track_name(lower, "mute");
        return (VoiceAction::TrackControl(TrackCommand::Mute(track)), 0.90);
    }
    if lower.contains("unmute") {
        let track = extract_track_name(lower, "unmute");
        return (VoiceAction::TrackControl(TrackCommand::Unmute(track)), 0.90);
    }

    // Solo/unsolo
    if lower.contains("solo") && !lower.contains("unsolo") {
        let track = extract_track_name(lower, "solo");
        return (VoiceAction::TrackControl(TrackCommand::Solo(track)), 0.90);
    }
    if lower.contains("unsolo") {
        let track = extract_track_name(lower, "unsolo");
        return (VoiceAction::TrackControl(TrackCommand::Unsolo(track)), 0.90);
    }

    // Volume
    if lower.contains("louder") || lower.contains("turn up") || lower.contains("volume up") {
        let track = extract_track_context(lower);
        return (
            VoiceAction::TrackControl(TrackCommand::Volume {
                track,
                direction: Direction::Up,
            }),
            0.80,
        );
    }
    if lower.contains("quieter")
        || lower.contains("turn down")
        || lower.contains("volume down")
        || lower.contains("softer")
    {
        let track = extract_track_context(lower);
        return (
            VoiceAction::TrackControl(TrackCommand::Volume {
                track,
                direction: Direction::Down,
            }),
            0.80,
        );
    }

    // Pan
    if lower.contains("pan left") || lower.contains("move left") {
        let track = extract_track_context(lower);
        return (
            VoiceAction::TrackControl(TrackCommand::Pan {
                track,
                direction: PanDirection::Left,
            }),
            0.85,
        );
    }
    if lower.contains("pan right") || lower.contains("move right") {
        let track = extract_track_context(lower);
        return (
            VoiceAction::TrackControl(TrackCommand::Pan {
                track,
                direction: PanDirection::Right,
            }),
            0.85,
        );
    }
    if lower.contains("pan center") || lower.contains("center pan") {
        let track = extract_track_context(lower);
        return (
            VoiceAction::TrackControl(TrackCommand::Pan {
                track,
                direction: PanDirection::Center,
            }),
            0.85,
        );
    }

    // Tempo
    if (lower.contains("set tempo") || lower.contains("set bpm") || lower.contains("tempo to"))
        && let Some(bpm) = extract_number(words)
    {
        return (VoiceAction::Tempo(TempoCommand::Set(bpm)), 0.90);
    }
    if matches_any(lower, &["faster", "speed up", "increase tempo"]) {
        return (VoiceAction::Tempo(TempoCommand::Faster), 0.80);
    }
    if matches_any(lower, &["slower", "slow down", "decrease tempo"]) {
        return (VoiceAction::Tempo(TempoCommand::Slower), 0.80);
    }

    // Auto-mix
    if matches_any(
        lower,
        &[
            "auto mix",
            "auto-mix",
            "automix",
            "mix it",
            "balance the mix",
        ],
    ) {
        return (VoiceAction::Mix(MixCommand::AutoMix), 0.85);
    }

    // Analysis
    if lower.contains("analyze") || lower.contains("analyse") || lower.contains("check") {
        if lower.contains("spectrum") || lower.contains("frequencies") {
            let track = extract_track_context(lower);
            return (VoiceAction::Analyze(AnalyzeCommand::Spectrum(track)), 0.80);
        }
        if lower.contains("dynamics") || lower.contains("levels") || lower.contains("loudness") {
            let track = extract_track_context(lower);
            return (VoiceAction::Analyze(AnalyzeCommand::Dynamics(track)), 0.80);
        }
        if lower.contains("mix") || lower.contains("everything") || lower.contains("all") {
            return (VoiceAction::Analyze(AnalyzeCommand::FullMix), 0.80);
        }
    }

    // Unknown
    (VoiceAction::Unknown(lower.to_string()), 0.0)
}

fn matches_any(input: &str, patterns: &[&str]) -> bool {
    patterns
        .iter()
        .any(|p| input == *p || input.starts_with(&format!("{} ", p)))
}

fn extract_number(words: &[&str]) -> Option<f64> {
    words.iter().find_map(|w| w.parse::<f64>().ok())
}

fn extract_track_name(input: &str, keyword: &str) -> String {
    // Try "mute the drums" or "mute drums"
    if let Some(pos) = input.find(keyword) {
        let after = input[pos + keyword.len()..].trim();
        let after = after.strip_prefix("the ").unwrap_or(after);
        let name = after.split_whitespace().next().unwrap_or("").to_string();
        if !name.is_empty() {
            return name;
        }
    }
    String::new()
}

fn extract_track_context(input: &str) -> String {
    // Look for patterns like "on the vocals", "the drums", "on vocals"
    let markers = ["on the ", "on ", "the ", "for "];
    for marker in &markers {
        if let Some(pos) = input.rfind(marker) {
            let after = &input[pos + marker.len()..];
            let name = after.split_whitespace().next().unwrap_or("").to_string();
            if !name.is_empty() && !["mix", "everything", "all", "it"].contains(&name.as_str()) {
                return name;
            }
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_play() {
        let intent = parse_voice_input("play");
        assert_eq!(
            intent.action,
            VoiceAction::Transport(TransportCommand::Play)
        );
        assert!(intent.confidence > 0.9);
    }

    #[test]
    fn parse_stop() {
        let intent = parse_voice_input("stop");
        assert_eq!(
            intent.action,
            VoiceAction::Transport(TransportCommand::Stop)
        );
    }

    #[test]
    fn parse_record() {
        let intent = parse_voice_input("start recording");
        assert_eq!(
            intent.action,
            VoiceAction::Transport(TransportCommand::Record)
        );
    }

    #[test]
    fn parse_seek_bar() {
        let intent = parse_voice_input("go to bar 16");
        assert_eq!(intent.action, VoiceAction::Seek(SeekTarget::Bar(16)));
    }

    #[test]
    fn parse_play_from_bar() {
        let intent = parse_voice_input("play from bar 8");
        assert_eq!(intent.action, VoiceAction::Seek(SeekTarget::Bar(8)));
    }

    #[test]
    fn parse_rewind() {
        let intent = parse_voice_input("go to the beginning");
        assert_eq!(intent.action, VoiceAction::Seek(SeekTarget::Beginning));
    }

    #[test]
    fn parse_mute_track() {
        let intent = parse_voice_input("mute the drums");
        assert_eq!(
            intent.action,
            VoiceAction::TrackControl(TrackCommand::Mute("drums".into()))
        );
    }

    #[test]
    fn parse_solo_track() {
        let intent = parse_voice_input("solo vocals");
        assert_eq!(
            intent.action,
            VoiceAction::TrackControl(TrackCommand::Solo("vocals".into()))
        );
    }

    #[test]
    fn parse_louder() {
        let intent = parse_voice_input("louder on the vocals");
        match &intent.action {
            VoiceAction::TrackControl(TrackCommand::Volume { track, direction }) => {
                assert_eq!(track, "vocals");
                assert_eq!(*direction, Direction::Up);
            }
            other => panic!("expected Volume, got {:?}", other),
        }
    }

    #[test]
    fn parse_set_tempo() {
        let intent = parse_voice_input("set tempo to 128");
        assert_eq!(intent.action, VoiceAction::Tempo(TempoCommand::Set(128.0)));
    }

    #[test]
    fn parse_faster() {
        let intent = parse_voice_input("faster");
        assert_eq!(intent.action, VoiceAction::Tempo(TempoCommand::Faster));
    }

    #[test]
    fn parse_auto_mix() {
        let intent = parse_voice_input("auto mix");
        assert_eq!(intent.action, VoiceAction::Mix(MixCommand::AutoMix));
    }

    #[test]
    fn parse_analyze_spectrum() {
        let intent = parse_voice_input("analyze the spectrum on vocals");
        match &intent.action {
            VoiceAction::Analyze(AnalyzeCommand::Spectrum(track)) => {
                assert_eq!(track, "vocals");
            }
            other => panic!("expected Spectrum, got {:?}", other),
        }
    }

    #[test]
    fn parse_unknown() {
        let intent = parse_voice_input("make me a sandwich");
        assert!(matches!(intent.action, VoiceAction::Unknown(_)));
        assert_eq!(intent.confidence, 0.0);
    }

    #[test]
    fn parse_pan_left() {
        let intent = parse_voice_input("pan left on the guitar");
        match &intent.action {
            VoiceAction::TrackControl(TrackCommand::Pan { track, direction }) => {
                assert_eq!(track, "guitar");
                assert_eq!(*direction, PanDirection::Left);
            }
            other => panic!("expected Pan, got {:?}", other),
        }
    }

    #[test]
    fn voice_intent_serializes() {
        let intent = parse_voice_input("play");
        let json = serde_json::to_string(&intent).unwrap();
        let back: VoiceIntent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.action, intent.action);
    }
}
