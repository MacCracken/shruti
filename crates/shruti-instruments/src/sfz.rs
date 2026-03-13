//! SFZ instrument definition file parser.
//!
//! Parses the SFZ text format and produces `SfzRegion` descriptors.
//! Because SFZ files reference external audio samples by path, the parser
//! returns metadata only — actual sample loading is the caller's
//! responsibility.

#![deny(unsafe_code)]

use crate::sampler::LoopMode;

/// A parsed SFZ `<region>` with all supported opcodes.
#[derive(Debug, Clone)]
pub struct SfzRegion {
    /// Relative path to the sample file (as written in the SFZ).
    pub sample_path: String,
    /// MIDI key range (low, high). Defaults to (0, 127).
    pub key_range: (u8, u8),
    /// MIDI velocity range (low, high). Defaults to (0, 127).
    pub vel_range: (u8, u8),
    /// Root key override (`pitch_keycenter`).
    pub root_key: Option<u8>,
    /// Loop mode override.
    pub loop_mode: Option<LoopMode>,
    /// Loop start sample offset.
    pub loop_start: Option<usize>,
    /// Loop end sample offset.
    pub loop_end: Option<usize>,
    /// Tuning offset in semitones.
    pub tune: Option<f32>,
    /// Volume in dB.
    pub volume: Option<f32>,
    /// Pan (-100..100).
    pub pan: Option<f32>,
}

impl Default for SfzRegion {
    fn default() -> Self {
        Self {
            sample_path: String::new(),
            key_range: (0, 127),
            vel_range: (0, 127),
            root_key: None,
            loop_mode: None,
            loop_start: None,
            loop_end: None,
            tune: None,
            volume: None,
            pan: None,
        }
    }
}

// ── Internal helpers ────────────────────────────────────────────────────

/// Intermediate accumulator while parsing opcodes.
#[derive(Debug, Clone, Default)]
struct OpcodeSet {
    sample: Option<String>,
    lokey: Option<u8>,
    hikey: Option<u8>,
    key: Option<u8>,
    lovel: Option<u8>,
    hivel: Option<u8>,
    pitch_keycenter: Option<u8>,
    loop_mode: Option<LoopMode>,
    loop_start: Option<usize>,
    loop_end: Option<usize>,
    tune: Option<f32>,
    volume: Option<f32>,
    pan: Option<f32>,
}

impl OpcodeSet {
    /// Apply a single `key=value` opcode.
    fn apply(&mut self, key: &str, value: &str) {
        match key {
            "sample" => self.sample = Some(value.to_string()),
            "lokey" => self.lokey = parse_note_or_number(value),
            "hikey" => self.hikey = parse_note_or_number(value),
            "key" => self.key = parse_note_or_number(value),
            "lovel" => self.lovel = value.parse().ok(),
            "hivel" => self.hivel = value.parse().ok(),
            "pitch_keycenter" => self.pitch_keycenter = parse_note_or_number(value),
            "loop_mode" | "loopmode" => {
                self.loop_mode = match value {
                    "no_loop" => Some(LoopMode::NoLoop),
                    "loop_continuous" | "loop_forward" => Some(LoopMode::Forward),
                    "loop_sustain" => Some(LoopMode::Forward),
                    "loop_pingpong" => Some(LoopMode::PingPong),
                    _ => None,
                };
            }
            "loop_start" | "loopstart" => self.loop_start = value.parse().ok(),
            "loop_end" | "loopend" => self.loop_end = value.parse().ok(),
            "tune" => self.tune = value.parse().ok(),
            "volume" => self.volume = value.parse().ok(),
            "pan" => self.pan = value.parse().ok(),
            _ => {} // ignore unknown opcodes
        }
    }

    /// Merge `self` on top of `defaults` (self wins where present).
    fn merge_with_defaults(&self, defaults: &OpcodeSet) -> OpcodeSet {
        OpcodeSet {
            sample: self.sample.clone().or_else(|| defaults.sample.clone()),
            lokey: self.lokey.or(defaults.lokey),
            hikey: self.hikey.or(defaults.hikey),
            key: self.key.or(defaults.key),
            lovel: self.lovel.or(defaults.lovel),
            hivel: self.hivel.or(defaults.hivel),
            pitch_keycenter: self.pitch_keycenter.or(defaults.pitch_keycenter),
            loop_mode: self.loop_mode.or(defaults.loop_mode),
            loop_start: self.loop_start.or(defaults.loop_start),
            loop_end: self.loop_end.or(defaults.loop_end),
            tune: self.tune.or(defaults.tune),
            volume: self.volume.or(defaults.volume),
            pan: self.pan.or(defaults.pan),
        }
    }

    /// Convert resolved opcodes into an `SfzRegion`.
    fn into_region(self) -> Option<SfzRegion> {
        let sample_path = self.sample?;
        if sample_path.is_empty() {
            return None;
        }

        // `key` is a shorthand that sets lokey, hikey, and pitch_keycenter.
        let (lo, hi) = if let Some(k) = self.key {
            (self.lokey.unwrap_or(k), self.hikey.unwrap_or(k))
        } else {
            (self.lokey.unwrap_or(0), self.hikey.unwrap_or(127))
        };

        let root = self.pitch_keycenter.or(self.key);

        Some(SfzRegion {
            sample_path,
            key_range: (lo, hi),
            vel_range: (self.lovel.unwrap_or(0), self.hivel.unwrap_or(127)),
            root_key: root,
            loop_mode: self.loop_mode,
            loop_start: self.loop_start,
            loop_end: self.loop_end,
            tune: self.tune,
            volume: self.volume,
            pan: self.pan,
        })
    }
}

/// Parse a MIDI note name (e.g. `c4`, `f#5`, `eb3`) or a plain number.
fn parse_note_or_number(s: &str) -> Option<u8> {
    // Try plain number first.
    if let Ok(n) = s.parse::<u8>() {
        return Some(n);
    }

    let s = s.to_lowercase();
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let (note_base, idx) = match bytes[0] {
        b'c' => (0i16, 1),
        b'd' => (2, 1),
        b'e' => (4, 1),
        b'f' => (5, 1),
        b'g' => (7, 1),
        b'a' => (9, 1),
        b'b' => (11, 1),
        _ => return None,
    };

    let (accidental, idx) = if idx < bytes.len() {
        match bytes[idx] {
            b'#' => (1i16, idx + 1),
            b'b' => (-1i16, idx + 1),
            _ => (0, idx),
        }
    } else {
        (0, idx)
    };

    let octave_str = &s[idx..];
    let octave: i16 = octave_str.parse().ok()?;

    // MIDI: C4 = 60
    let midi = (octave + 1) * 12 + note_base + accidental;
    if (0..=127).contains(&midi) {
        Some(midi as u8)
    } else {
        None
    }
}

// ── Token types used during parsing ─────────────────────────────────────

#[derive(Debug, PartialEq)]
enum Header {
    Global,
    Group,
    Region,
    Control,
    Other(String),
}

/// Strip a line of its trailing comment (SFZ uses `//` comments).
fn strip_comment(line: &str) -> &str {
    match line.find("//") {
        Some(pos) => &line[..pos],
        None => line,
    }
}

/// Parse all `<header>` tags and opcodes from a single stripped line.
/// Returns a list of (Option<Header>, Vec<(key, value)>) segments.
type LineSegment<'a> = (Option<Header>, Vec<(&'a str, &'a str)>);

fn tokenize_line(line: &str) -> Vec<LineSegment<'_>> {
    let mut segments: Vec<LineSegment<'_>> = Vec::new();
    let mut rest = line;

    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }

        // Check for a <header> tag.
        if rest.starts_with('<')
            && let Some(end) = rest.find('>')
        {
            let tag = &rest[1..end];
            let header = match tag {
                "global" => Header::Global,
                "group" => Header::Group,
                "region" => Header::Region,
                "control" => Header::Control,
                other => Header::Other(other.to_string()),
            };
            segments.push((Some(header), Vec::new()));
            rest = &rest[end + 1..];
            continue;
        }

        // Otherwise, parse opcodes until the next `<` or end of line.
        let chunk_end = rest.find('<').unwrap_or(rest.len());
        let chunk = &rest[..chunk_end];

        // Parse key=value pairs from the chunk.
        let opcodes = parse_opcodes(chunk);

        if !opcodes.is_empty() {
            if let Some(last) = segments.last_mut() {
                last.1.extend(opcodes);
            } else {
                // Opcodes before any header — treat as global.
                segments.push((None, opcodes));
            }
        }

        rest = &rest[chunk_end..];
    }

    segments
}

/// Parse `key=value` pairs from a chunk of text.
///
/// SFZ opcodes are space-separated, but the `sample` opcode value can
/// contain spaces (it runs to end-of-chunk or next `=`-bearing token).
fn parse_opcodes(chunk: &str) -> Vec<(&str, &str)> {
    let mut result = Vec::new();
    let chunk = chunk.trim();
    if chunk.is_empty() {
        return result;
    }

    // Find all `=` positions and work out which tokens are keys.
    let tokens: Vec<&str> = chunk.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        if let Some(eq) = tokens[i].find('=') {
            let key = &tokens[i][..eq];
            let val_start = &tokens[i][eq + 1..];

            // For `sample=`, the value extends to the next opcode key.
            if key == "sample" {
                // Collect tokens until the next key=value or end.
                let mut parts = vec![val_start];
                let mut j = i + 1;
                while j < tokens.len() {
                    if tokens[j].contains('=') {
                        break;
                    }
                    parts.push(tokens[j]);
                    j += 1;
                }
                // Reconstruct the value from original chunk offsets for
                // multi-word sample paths.
                let full_val = parts.join(" ");
                // We need to return references into the chunk, but joining
                // creates an owned string. Instead, find the span in chunk.
                let val_offset = chunk
                    .find(&format!("sample={}", parts[0]))
                    .map(|p| p + "sample=".len());
                if let Some(start) = val_offset {
                    let end = if j < tokens.len() {
                        // Find the start of the next token in chunk.
                        chunk.find(tokens[j]).unwrap_or(chunk.len())
                    } else {
                        chunk.len()
                    };
                    let val = chunk[start..end].trim();
                    result.push((key, val));
                } else if !full_val.is_empty() {
                    // Fallback — push the first part only.
                    result.push((key, val_start));
                }
                i = j;
            } else {
                result.push((key, val_start));
                i += 1;
            }
        } else {
            // Not a key=value token, skip.
            i += 1;
        }
    }

    result
}

// ── Public API ──────────────────────────────────────────────────────────

/// Parse an SFZ file's text content into a list of regions.
///
/// `<group>` opcodes serve as defaults for all subsequent `<region>`
/// blocks until the next `<group>` (or `<global>`).  `<global>` opcodes
/// serve as defaults for every region in the file.
pub fn parse_sfz(content: &str) -> Result<Vec<SfzRegion>, String> {
    let mut global = OpcodeSet::default();
    let mut group = OpcodeSet::default();
    let mut current_region: Option<OpcodeSet> = None;
    let mut regions: Vec<SfzRegion> = Vec::new();

    // Track what the current header context is.
    #[derive(PartialEq)]
    enum Context {
        None,
        Global,
        Group,
        Region,
    }
    let mut ctx = Context::None;

    for line in content.lines() {
        let stripped = strip_comment(line);
        let segments = tokenize_line(stripped);

        for (header, opcodes) in segments {
            if let Some(h) = header {
                match h {
                    Header::Global => {
                        // Flush any pending region.
                        if let Some(region_ops) = current_region.take() {
                            let merged = region_ops.merge_with_defaults(&group);
                            let merged = merged.merge_with_defaults(&global);
                            if let Some(r) = merged.into_region() {
                                regions.push(r);
                            }
                        }
                        global = OpcodeSet::default();
                        group = OpcodeSet::default();
                        ctx = Context::Global;
                    }
                    Header::Group => {
                        // Flush any pending region.
                        if let Some(region_ops) = current_region.take() {
                            let merged = region_ops.merge_with_defaults(&group);
                            let merged = merged.merge_with_defaults(&global);
                            if let Some(r) = merged.into_region() {
                                regions.push(r);
                            }
                        }
                        group = OpcodeSet::default();
                        ctx = Context::Group;
                    }
                    Header::Region => {
                        // Flush any pending region.
                        if let Some(region_ops) = current_region.take() {
                            let merged = region_ops.merge_with_defaults(&group);
                            let merged = merged.merge_with_defaults(&global);
                            if let Some(r) = merged.into_region() {
                                regions.push(r);
                            }
                        }
                        current_region = Some(OpcodeSet::default());
                        ctx = Context::Region;
                    }
                    Header::Control | Header::Other(_) => {
                        // Flush pending region, ignore these headers.
                        if let Some(region_ops) = current_region.take() {
                            let merged = region_ops.merge_with_defaults(&group);
                            let merged = merged.merge_with_defaults(&global);
                            if let Some(r) = merged.into_region() {
                                regions.push(r);
                            }
                        }
                        ctx = Context::None;
                    }
                }
            }

            // Apply opcodes to the current context.
            for (key, value) in &opcodes {
                match ctx {
                    Context::Global => global.apply(key, value),
                    Context::Group => group.apply(key, value),
                    Context::Region => {
                        if let Some(ref mut r) = current_region {
                            r.apply(key, value);
                        }
                    }
                    Context::None => {}
                }
            }
        }
    }

    // Flush final pending region.
    if let Some(region_ops) = current_region.take() {
        let merged = region_ops.merge_with_defaults(&group);
        let merged = merged.merge_with_defaults(&global);
        if let Some(r) = merged.into_region() {
            regions.push(r);
        }
    }

    Ok(regions)
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_region() {
        let sfz = "<region> sample=piano_c4.wav lokey=48 hikey=72 pitch_keycenter=60\n";
        let regions = parse_sfz(sfz).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].sample_path, "piano_c4.wav");
        assert_eq!(regions[0].key_range, (48, 72));
        assert_eq!(regions[0].root_key, Some(60));
    }

    #[test]
    fn parse_multiple_regions() {
        let sfz = "\
<region> sample=low.wav lokey=0 hikey=59 pitch_keycenter=48
<region> sample=high.wav lokey=60 hikey=127 pitch_keycenter=72
";
        let regions = parse_sfz(sfz).unwrap();
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].sample_path, "low.wav");
        assert_eq!(regions[1].sample_path, "high.wav");
        assert_eq!(regions[0].key_range, (0, 59));
        assert_eq!(regions[1].key_range, (60, 127));
    }

    #[test]
    fn parse_group_defaults() {
        let sfz = "\
<group> lovel=64 hivel=127
<region> sample=a.wav lokey=0 hikey=63
<region> sample=b.wav lokey=64 hikey=127
";
        let regions = parse_sfz(sfz).unwrap();
        assert_eq!(regions.len(), 2);
        // Both inherit velocity from group.
        assert_eq!(regions[0].vel_range, (64, 127));
        assert_eq!(regions[1].vel_range, (64, 127));
    }

    #[test]
    fn parse_global_defaults() {
        let sfz = "\
<global> pitch_keycenter=60 volume=-6
<region> sample=a.wav
<region> sample=b.wav pitch_keycenter=72
";
        let regions = parse_sfz(sfz).unwrap();
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].root_key, Some(60));
        assert_eq!(regions[0].volume, Some(-6.0));
        // Region-level override wins.
        assert_eq!(regions[1].root_key, Some(72));
    }

    #[test]
    fn parse_velocity_ranges() {
        let sfz = "\
<region> sample=soft.wav lovel=0 hivel=63
<region> sample=loud.wav lovel=64 hivel=127
";
        let regions = parse_sfz(sfz).unwrap();
        assert_eq!(regions[0].vel_range, (0, 63));
        assert_eq!(regions[1].vel_range, (64, 127));
    }

    #[test]
    fn parse_loop_opcodes() {
        let sfz =
            "<region> sample=pad.wav loop_mode=loop_continuous loop_start=1000 loop_end=5000\n";
        let regions = parse_sfz(sfz).unwrap();
        assert_eq!(regions[0].loop_mode, Some(LoopMode::Forward));
        assert_eq!(regions[0].loop_start, Some(1000));
        assert_eq!(regions[0].loop_end, Some(5000));
    }

    #[test]
    fn parse_pingpong_loop() {
        let sfz = "<region> sample=pad.wav loop_mode=loop_pingpong\n";
        let regions = parse_sfz(sfz).unwrap();
        assert_eq!(regions[0].loop_mode, Some(LoopMode::PingPong));
    }

    #[test]
    fn parse_comments_ignored() {
        let sfz = "\
// This is a comment
<region> sample=test.wav // inline comment
// <region> sample=ghost.wav
";
        let regions = parse_sfz(sfz).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].sample_path, "test.wav");
    }

    #[test]
    fn parse_empty_input() {
        let regions = parse_sfz("").unwrap();
        assert!(regions.is_empty());
    }

    #[test]
    fn parse_whitespace_only() {
        let regions = parse_sfz("   \n\n  \n").unwrap();
        assert!(regions.is_empty());
    }

    #[test]
    fn parse_region_without_sample_skipped() {
        let sfz = "<region> lokey=0 hikey=127\n";
        let regions = parse_sfz(sfz).unwrap();
        assert!(regions.is_empty());
    }

    #[test]
    fn parse_key_shorthand() {
        let sfz = "<region> sample=snare.wav key=38\n";
        let regions = parse_sfz(sfz).unwrap();
        assert_eq!(regions[0].key_range, (38, 38));
        assert_eq!(regions[0].root_key, Some(38));
    }

    #[test]
    fn parse_note_names() {
        assert_eq!(parse_note_or_number("c4"), Some(60));
        assert_eq!(parse_note_or_number("a4"), Some(69));
        assert_eq!(parse_note_or_number("f#3"), Some(54));
        assert_eq!(parse_note_or_number("eb4"), Some(63));
        assert_eq!(parse_note_or_number("60"), Some(60));
    }

    #[test]
    fn parse_tune_and_pan() {
        let sfz = "<region> sample=test.wav tune=50 pan=-30\n";
        let regions = parse_sfz(sfz).unwrap();
        assert_eq!(regions[0].tune, Some(50.0));
        assert_eq!(regions[0].pan, Some(-30.0));
    }

    #[test]
    fn parse_multiple_headers_one_line() {
        let sfz = "<group> lovel=0 hivel=63 <region> sample=a.wav <region> sample=b.wav\n";
        let regions = parse_sfz(sfz).unwrap();
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].vel_range, (0, 63));
        assert_eq!(regions[1].vel_range, (0, 63));
    }

    #[test]
    fn group_reset_between_groups() {
        let sfz = "\
<group> lovel=0 hivel=63
<region> sample=soft.wav
<group> lovel=64 hivel=127
<region> sample=loud.wav
";
        let regions = parse_sfz(sfz).unwrap();
        assert_eq!(regions[0].vel_range, (0, 63));
        assert_eq!(regions[1].vel_range, (64, 127));
    }
}
