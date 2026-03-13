//! SoundFont 2 (SF2) binary file parser.
//!
//! Parses the RIFF-based SF2 format and extracts presets with their
//! sample zones.  The PCM sample data embedded in the `sdta` chunk is
//! converted to `f32` and stored directly in `SampleZone`.

#![deny(unsafe_code)]

use crate::sampler::{LoopMode, SampleZone};

// ── Public types ────────────────────────────────────────────────────────

/// A parsed SF2 preset containing its name and sample zones.
#[derive(Debug, Clone)]
pub struct Sf2Preset {
    pub name: String,
    pub bank: u16,
    pub preset_number: u16,
    pub zones: Vec<SampleZone>,
}

// ── RIFF / SF2 constants ────────────────────────────────────────────────

const RIFF_ID: &[u8; 4] = b"RIFF";
const SFBK_ID: &[u8; 4] = b"sfbk";
const LIST_ID: &[u8; 4] = b"LIST";

// Sub-chunk IDs within pdta
const PHDR_ID: &[u8; 4] = b"phdr";
const PBAG_ID: &[u8; 4] = b"pbag";
const PGEN_ID: &[u8; 4] = b"pgen";
const INST_ID: &[u8; 4] = b"inst";
const IBAG_ID: &[u8; 4] = b"ibag";
const IGEN_ID: &[u8; 4] = b"igen";
const SHDR_ID: &[u8; 4] = b"shdr";

// Generator operators we care about
const GEN_KEY_RANGE: u16 = 43;
const GEN_VEL_RANGE: u16 = 44;
const GEN_INSTRUMENT: u16 = 41;
const GEN_SAMPLE_ID: u16 = 53;
const GEN_OVERRIDING_ROOT_KEY: u16 = 58;
const GEN_SAMPLE_MODES: u16 = 54;

// ── Raw SF2 record structs ──────────────────────────────────────────────

#[derive(Debug, Clone)]
struct PhdrRecord {
    name: String,
    preset: u16,
    bank: u16,
    bag_index: u16,
}

#[derive(Debug, Clone, Copy)]
struct BagRecord {
    gen_index: u16,
    _mod_index: u16,
}

#[derive(Debug, Clone, Copy)]
struct GenRecord {
    oper: u16,
    amount: i16,
}

impl GenRecord {
    fn amount_range(&self) -> (u8, u8) {
        let lo = (self.amount & 0xFF) as u8;
        let hi = ((self.amount >> 8) & 0xFF) as u8;
        (lo, hi)
    }
}

#[derive(Debug, Clone)]
struct InstRecord {
    #[allow(dead_code)] // Part of SF2 spec; useful for diagnostics.
    name: String,
    bag_index: u16,
}

#[derive(Debug, Clone)]
struct ShdrRecord {
    name: String,
    start: u32,
    end: u32,
    loop_start: u32,
    loop_end: u32,
    sample_rate: u32,
    original_pitch: u8,
    _pitch_correction: i8,
    _sample_link: u16,
    sample_type: u16,
}

// ── Low-level reading helpers ───────────────────────────────────────────

fn read_u8(data: &[u8], offset: usize) -> Result<u8, String> {
    data.get(offset)
        .copied()
        .ok_or_else(|| format!("unexpected end of data at offset {offset}"))
}

fn read_i8(data: &[u8], offset: usize) -> Result<i8, String> {
    read_u8(data, offset).map(|b| b as i8)
}

fn read_u16_le(data: &[u8], offset: usize) -> Result<u16, String> {
    if offset + 2 > data.len() {
        return Err(format!("unexpected end of data at offset {offset}"));
    }
    Ok(u16::from_le_bytes([data[offset], data[offset + 1]]))
}

fn read_i16_le(data: &[u8], offset: usize) -> Result<i16, String> {
    read_u16_le(data, offset).map(|v| v as i16)
}

fn read_u32_le(data: &[u8], offset: usize) -> Result<u32, String> {
    if offset + 4 > data.len() {
        return Err(format!("unexpected end of data at offset {offset}"));
    }
    Ok(u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

fn read_fourcc(data: &[u8], offset: usize) -> Result<[u8; 4], String> {
    if offset + 4 > data.len() {
        return Err(format!("unexpected end of data at offset {offset}"));
    }
    let mut cc = [0u8; 4];
    cc.copy_from_slice(&data[offset..offset + 4]);
    Ok(cc)
}

fn read_fixed_string(data: &[u8], offset: usize, len: usize) -> Result<String, String> {
    if offset + len > data.len() {
        return Err(format!("unexpected end of data at offset {offset}"));
    }
    let slice = &data[offset..offset + len];
    // Trim trailing NULs.
    let end = slice.iter().position(|&b| b == 0).unwrap_or(len);
    Ok(String::from_utf8_lossy(&slice[..end]).to_string())
}

// ── Chunk iteration ─────────────────────────────────────────────────────

/// A parsed RIFF chunk.
struct Chunk<'a> {
    id: [u8; 4],
    data: &'a [u8],
}

/// Iterate over sub-chunks within a slice (starting just after a LIST
/// form-type or RIFF form-type).
fn iter_chunks(data: &[u8]) -> ChunkIter<'_> {
    ChunkIter { data, offset: 0 }
}

struct ChunkIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for ChunkIter<'a> {
    type Item = Result<Chunk<'a>, String>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 8 > self.data.len() {
            return None;
        }
        let id = match read_fourcc(self.data, self.offset) {
            Ok(cc) => cc,
            Err(e) => return Some(Err(e)),
        };
        let size = match read_u32_le(self.data, self.offset + 4) {
            Ok(s) => s as usize,
            Err(e) => return Some(Err(e)),
        };
        let data_start = self.offset + 8;
        let data_end = data_start + size;
        if data_end > self.data.len() {
            return Some(Err(format!(
                "chunk {:?} at offset {} extends beyond data (size={size}, available={})",
                String::from_utf8_lossy(&id),
                self.offset,
                self.data.len() - data_start,
            )));
        }
        let chunk = Chunk {
            id,
            data: &self.data[data_start..data_end],
        };
        // Advance, pad to even boundary.
        self.offset = data_end + (size & 1);
        Some(Ok(chunk))
    }
}

// ── Record parsers ──────────────────────────────────────────────────────

fn parse_phdr_records(data: &[u8]) -> Result<Vec<PhdrRecord>, String> {
    const RECORD_SIZE: usize = 38;
    let count = data.len() / RECORD_SIZE;
    let mut records = Vec::with_capacity(count);
    for i in 0..count {
        let off = i * RECORD_SIZE;
        records.push(PhdrRecord {
            name: read_fixed_string(data, off, 20)?,
            preset: read_u16_le(data, off + 20)?,
            bank: read_u16_le(data, off + 22)?,
            bag_index: read_u16_le(data, off + 24)?,
            // skip preset_bag_ndx library, genre, morphology
        });
    }
    Ok(records)
}

fn parse_bag_records(data: &[u8]) -> Result<Vec<BagRecord>, String> {
    const RECORD_SIZE: usize = 4;
    let count = data.len() / RECORD_SIZE;
    let mut records = Vec::with_capacity(count);
    for i in 0..count {
        let off = i * RECORD_SIZE;
        records.push(BagRecord {
            gen_index: read_u16_le(data, off)?,
            _mod_index: read_u16_le(data, off + 2)?,
        });
    }
    Ok(records)
}

fn parse_gen_records(data: &[u8]) -> Result<Vec<GenRecord>, String> {
    const RECORD_SIZE: usize = 4;
    let count = data.len() / RECORD_SIZE;
    let mut records = Vec::with_capacity(count);
    for i in 0..count {
        let off = i * RECORD_SIZE;
        records.push(GenRecord {
            oper: read_u16_le(data, off)?,
            amount: read_i16_le(data, off + 2)?,
        });
    }
    Ok(records)
}

fn parse_inst_records(data: &[u8]) -> Result<Vec<InstRecord>, String> {
    const RECORD_SIZE: usize = 22;
    let count = data.len() / RECORD_SIZE;
    let mut records = Vec::with_capacity(count);
    for i in 0..count {
        let off = i * RECORD_SIZE;
        records.push(InstRecord {
            name: read_fixed_string(data, off, 20)?,
            bag_index: read_u16_le(data, off + 20)?,
        });
    }
    Ok(records)
}

fn parse_shdr_records(data: &[u8]) -> Result<Vec<ShdrRecord>, String> {
    const RECORD_SIZE: usize = 46;
    let count = data.len() / RECORD_SIZE;
    let mut records = Vec::with_capacity(count);
    for i in 0..count {
        let off = i * RECORD_SIZE;
        records.push(ShdrRecord {
            name: read_fixed_string(data, off, 20)?,
            start: read_u32_le(data, off + 20)?,
            end: read_u32_le(data, off + 24)?,
            loop_start: read_u32_le(data, off + 28)?,
            loop_end: read_u32_le(data, off + 32)?,
            sample_rate: read_u32_le(data, off + 36)?,
            original_pitch: read_u8(data, off + 40)?,
            _pitch_correction: read_i8(data, off + 41)?,
            _sample_link: read_u16_le(data, off + 42)?,
            sample_type: read_u16_le(data, off + 44)?,
        });
    }
    Ok(records)
}

// ── Sample extraction ───────────────────────────────────────────────────

/// Convert 16-bit signed PCM samples to f32 in [-1.0, 1.0].
fn pcm16_to_f32(data: &[u8], start_sample: usize, end_sample: usize) -> Vec<f32> {
    let byte_start = start_sample * 2;
    let byte_end = end_sample * 2;
    if byte_end > data.len() || byte_start > byte_end {
        return Vec::new();
    }
    let slice = &data[byte_start..byte_end];
    let num_samples = (byte_end - byte_start) / 2;
    let mut out = Vec::with_capacity(num_samples);
    for i in 0..num_samples {
        let off = i * 2;
        let sample = i16::from_le_bytes([slice[off], slice[off + 1]]);
        out.push(sample as f32 / 32768.0);
    }
    out
}

// ── Public API ──────────────────────────────────────────────────────────

/// Parse an SF2 (SoundFont 2) file from raw bytes.
///
/// Returns a list of `Sf2Preset`, each containing resolved `SampleZone`s
/// with the PCM data extracted and converted to f32.
pub fn parse_sf2(data: &[u8]) -> Result<Vec<Sf2Preset>, String> {
    // ── 1. Validate RIFF/sfbk outer container ───────────────────────
    if data.len() < 12 {
        return Err("file too small to be a valid SF2".to_string());
    }
    let riff_id = read_fourcc(data, 0)?;
    if riff_id != *RIFF_ID {
        return Err("not a RIFF file".to_string());
    }
    let _file_size = read_u32_le(data, 4)?;
    let form_type = read_fourcc(data, 8)?;
    if form_type != *SFBK_ID {
        return Err(format!(
            "RIFF form type is {:?}, expected 'sfbk'",
            String::from_utf8_lossy(&form_type)
        ));
    }

    // ── 2. Find the three required LIST chunks: INFO, sdta, pdta ────
    let mut sdta_smpl_data: Option<&[u8]> = None;
    let mut pdta_data: Option<&[u8]> = None;

    for chunk_result in iter_chunks(&data[12..]) {
        let chunk = chunk_result?;
        if chunk.id == *LIST_ID && chunk.data.len() >= 4 {
            let list_type = read_fourcc(chunk.data, 0)?;
            match &list_type {
                b"sdta" => {
                    // Find the smpl sub-chunk.
                    for sub_result in iter_chunks(&chunk.data[4..]) {
                        let sub = sub_result?;
                        if &sub.id == b"smpl" {
                            sdta_smpl_data = Some(sub.data);
                        }
                    }
                }
                b"pdta" => {
                    pdta_data = Some(&chunk.data[4..]);
                }
                _ => {}
            }
        }
    }

    let smpl_data = sdta_smpl_data.ok_or_else(|| "missing sdta/smpl chunk".to_string())?;
    let pdta = pdta_data.ok_or_else(|| "missing pdta chunk".to_string())?;

    // ── 3. Parse all pdta sub-chunks ────────────────────────────────
    let mut phdr_data: Option<&[u8]> = None;
    let mut pbag_data: Option<&[u8]> = None;
    let mut pgen_data: Option<&[u8]> = None;
    let mut inst_data: Option<&[u8]> = None;
    let mut ibag_data: Option<&[u8]> = None;
    let mut igen_data: Option<&[u8]> = None;
    let mut shdr_data: Option<&[u8]> = None;

    for chunk_result in iter_chunks(pdta) {
        let chunk = chunk_result?;
        match &chunk.id {
            id if id == PHDR_ID => phdr_data = Some(chunk.data),
            id if id == PBAG_ID => pbag_data = Some(chunk.data),
            id if id == PGEN_ID => pgen_data = Some(chunk.data),
            id if id == INST_ID => inst_data = Some(chunk.data),
            id if id == IBAG_ID => ibag_data = Some(chunk.data),
            id if id == IGEN_ID => igen_data = Some(chunk.data),
            id if id == SHDR_ID => shdr_data = Some(chunk.data),
            _ => {}
        }
    }

    let phdrs = parse_phdr_records(phdr_data.ok_or("missing phdr")?)?;
    let pbags = parse_bag_records(pbag_data.ok_or("missing pbag")?)?;
    let pgens = parse_gen_records(pgen_data.ok_or("missing pgen")?)?;
    let insts = parse_inst_records(inst_data.ok_or("missing inst")?)?;
    let ibags = parse_bag_records(ibag_data.ok_or("missing ibag")?)?;
    let igens = parse_gen_records(igen_data.ok_or("missing igen")?)?;
    let shdrs = parse_shdr_records(shdr_data.ok_or("missing shdr")?)?;

    // ── 4. Resolve presets → instruments → sample zones ─────────────
    let mut presets = Vec::new();

    // Last phdr is the terminal EOP record.
    for pi in 0..phdrs.len().saturating_sub(1) {
        let phdr = &phdrs[pi];
        let bag_start = phdr.bag_index as usize;
        let bag_end = phdrs[pi + 1].bag_index as usize;

        let mut zones: Vec<SampleZone> = Vec::new();

        for bi in bag_start..bag_end {
            if bi >= pbags.len() {
                break;
            }
            let gen_start = pbags[bi].gen_index as usize;
            let gen_end = if bi + 1 < pbags.len() {
                pbags[bi + 1].gen_index as usize
            } else {
                pgens.len()
            };

            // Find the instrument generator for this preset zone.
            let mut inst_index: Option<usize> = None;
            let mut preset_key_range: Option<(u8, u8)> = None;
            let mut preset_vel_range: Option<(u8, u8)> = None;

            for pg in &pgens[gen_start..gen_end.min(pgens.len())] {
                match pg.oper {
                    GEN_INSTRUMENT => inst_index = Some(pg.amount as usize),
                    GEN_KEY_RANGE => preset_key_range = Some(pg.amount_range()),
                    GEN_VEL_RANGE => preset_vel_range = Some(pg.amount_range()),
                    _ => {}
                }
            }

            let Some(ii) = inst_index else {
                continue; // global zone or no instrument — skip
            };
            if ii >= insts.len().saturating_sub(1) {
                continue;
            }

            // Resolve the instrument's zones.
            let inst = &insts[ii];
            let ibag_start = inst.bag_index as usize;
            let ibag_end = insts[ii + 1].bag_index as usize;

            for ib in ibag_start..ibag_end {
                if ib >= ibags.len() {
                    break;
                }
                let igen_start = ibags[ib].gen_index as usize;
                let igen_end = if ib + 1 < ibags.len() {
                    ibags[ib + 1].gen_index as usize
                } else {
                    igens.len()
                };

                let mut sample_id: Option<usize> = None;
                let mut key_range: (u8, u8) = (0, 127);
                let mut vel_range: (u8, u8) = (0, 127);
                let mut root_key_override: Option<u8> = None;
                let mut sample_modes: u16 = 0;

                for ig in &igens[igen_start..igen_end.min(igens.len())] {
                    match ig.oper {
                        GEN_SAMPLE_ID => sample_id = Some(ig.amount as usize),
                        GEN_KEY_RANGE => key_range = ig.amount_range(),
                        GEN_VEL_RANGE => vel_range = ig.amount_range(),
                        GEN_OVERRIDING_ROOT_KEY => {
                            let k = ig.amount as u8;
                            if k <= 127 {
                                root_key_override = Some(k);
                            }
                        }
                        GEN_SAMPLE_MODES => sample_modes = ig.amount as u16,
                        _ => {}
                    }
                }

                let Some(sid) = sample_id else {
                    continue; // global instrument zone
                };
                // Skip terminal sample header record.
                if sid >= shdrs.len().saturating_sub(1) {
                    continue;
                }
                let shdr = &shdrs[sid];

                // Skip ROM samples and linked samples we can't resolve.
                if shdr.sample_type & 0x8000 != 0 {
                    continue;
                }

                let root_key = root_key_override.unwrap_or(shdr.original_pitch);
                let loop_mode = match sample_modes & 3 {
                    0 => LoopMode::NoLoop,
                    1 | 2 => LoopMode::Forward,
                    3 => LoopMode::PingPong,
                    _ => LoopMode::NoLoop,
                };

                // Apply preset-level key/vel range restriction.
                let final_key = if let Some(pk) = preset_key_range {
                    (key_range.0.max(pk.0), key_range.1.min(pk.1))
                } else {
                    key_range
                };
                let final_vel = if let Some(pv) = preset_vel_range {
                    (vel_range.0.max(pv.0), vel_range.1.min(pv.1))
                } else {
                    vel_range
                };

                // Extract PCM data.
                let samples = pcm16_to_f32(smpl_data, shdr.start as usize, shdr.end as usize);

                // Use saturating_sub to prevent underflow when loop
                // points precede the sample start in malformed files.
                let loop_start = if loop_mode != LoopMode::NoLoop {
                    Some(shdr.loop_start.saturating_sub(shdr.start) as usize)
                } else {
                    None
                };
                let loop_end = if loop_mode != LoopMode::NoLoop {
                    Some(shdr.loop_end.saturating_sub(shdr.start) as usize)
                } else {
                    None
                };

                zones.push(SampleZone {
                    name: shdr.name.clone(),
                    root_key,
                    key_low: final_key.0,
                    key_high: final_key.1,
                    velocity_low: final_vel.0,
                    velocity_high: final_vel.1,
                    samples,
                    sample_rate: shdr.sample_rate,
                    loop_start,
                    loop_end,
                    loop_mode,
                    slices: Vec::new(),
                });
            }
        }

        // Skip terminal EOP preset and empty presets.
        if !zones.is_empty() {
            presets.push(Sf2Preset {
                name: phdr.name.clone(),
                bank: phdr.bank,
                preset_number: phdr.preset,
                zones,
            });
        }
    }

    Ok(presets)
}

// ── Test helpers: build synthetic SF2 binary data ───────────────────────

#[cfg(test)]
mod test_helpers {
    //! Helpers to construct minimal valid SF2 binary blobs for testing.

    pub fn write_u16_le(buf: &mut Vec<u8>, v: u16) {
        buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_u32_le(buf: &mut Vec<u8>, v: u32) {
        buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_i16_le(buf: &mut Vec<u8>, v: i16) {
        buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_fourcc(buf: &mut Vec<u8>, cc: &[u8; 4]) {
        buf.extend_from_slice(cc);
    }

    pub fn write_fixed_string(buf: &mut Vec<u8>, s: &str, len: usize) {
        let bytes = s.as_bytes();
        let copy_len = bytes.len().min(len);
        buf.extend_from_slice(&bytes[..copy_len]);
        for _ in copy_len..len {
            buf.push(0);
        }
    }

    /// Build a sub-chunk: id(4) + size(4) + data, padded to even.
    pub fn make_chunk(id: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        write_fourcc(&mut buf, id);
        write_u32_le(&mut buf, data.len() as u32);
        buf.extend_from_slice(data);
        if !data.len().is_multiple_of(2) {
            buf.push(0);
        }
        buf
    }

    /// Build a LIST chunk: LIST + size + form_type + sub-chunks.
    pub fn make_list(form_type: &[u8; 4], sub_chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut inner = Vec::new();
        inner.extend_from_slice(form_type);
        for sc in sub_chunks {
            inner.extend_from_slice(sc);
        }
        let mut buf = Vec::new();
        write_fourcc(&mut buf, b"LIST");
        write_u32_le(&mut buf, inner.len() as u32);
        buf.extend_from_slice(&inner);
        buf
    }

    /// Build a complete RIFF/sfbk file from three LIST chunks.
    pub fn make_sf2(info: Vec<u8>, sdta: Vec<u8>, pdta: Vec<u8>) -> Vec<u8> {
        let mut inner = Vec::new();
        inner.extend_from_slice(b"sfbk");
        inner.extend_from_slice(&info);
        inner.extend_from_slice(&sdta);
        inner.extend_from_slice(&pdta);
        let mut buf = Vec::new();
        write_fourcc(&mut buf, b"RIFF");
        write_u32_le(&mut buf, inner.len() as u32);
        buf.extend_from_slice(&inner);
        buf
    }

    // ── Record builders ─────────────────────────────────────────────

    /// phdr record (38 bytes).
    pub fn make_phdr(name: &str, preset: u16, bank: u16, bag_ndx: u16) -> Vec<u8> {
        let mut buf = Vec::new();
        write_fixed_string(&mut buf, name, 20);
        write_u16_le(&mut buf, preset);
        write_u16_le(&mut buf, bank);
        write_u16_le(&mut buf, bag_ndx); // preset bag index
        write_u32_le(&mut buf, 0); // library
        write_u32_le(&mut buf, 0); // genre
        write_u32_le(&mut buf, 0); // morphology
        assert_eq!(buf.len(), 38);
        buf
    }

    /// pbag / ibag record (4 bytes).
    pub fn make_bag(gen_ndx: u16, mod_ndx: u16) -> Vec<u8> {
        let mut buf = Vec::new();
        write_u16_le(&mut buf, gen_ndx);
        write_u16_le(&mut buf, mod_ndx);
        buf
    }

    /// gen record (4 bytes).
    pub fn make_gen(oper: u16, amount: i16) -> Vec<u8> {
        let mut buf = Vec::new();
        write_u16_le(&mut buf, oper);
        write_i16_le(&mut buf, amount);
        buf
    }

    /// gen record with range amount (lo in low byte, hi in high byte).
    pub fn make_gen_range(oper: u16, lo: u8, hi: u8) -> Vec<u8> {
        let amount = (lo as i16) | ((hi as i16) << 8);
        make_gen(oper, amount)
    }

    /// inst record (22 bytes).
    pub fn make_inst(name: &str, bag_ndx: u16) -> Vec<u8> {
        let mut buf = Vec::new();
        write_fixed_string(&mut buf, name, 20);
        write_u16_le(&mut buf, bag_ndx);
        buf
    }

    /// shdr record (46 bytes).
    #[allow(clippy::too_many_arguments)]
    pub fn make_shdr(
        name: &str,
        start: u32,
        end: u32,
        loop_start: u32,
        loop_end: u32,
        sample_rate: u32,
        original_pitch: u8,
        sample_type: u16,
    ) -> Vec<u8> {
        let mut buf = Vec::new();
        write_fixed_string(&mut buf, name, 20);
        write_u32_le(&mut buf, start);
        write_u32_le(&mut buf, end);
        write_u32_le(&mut buf, loop_start);
        write_u32_le(&mut buf, loop_end);
        write_u32_le(&mut buf, sample_rate);
        buf.push(original_pitch);
        buf.push(0); // pitch correction
        write_u16_le(&mut buf, 0); // sample link
        write_u16_le(&mut buf, sample_type);
        assert_eq!(buf.len(), 46);
        buf
    }

    /// Build 16-bit PCM sample data from f32 values.
    pub fn make_pcm16(samples: &[f32]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(samples.len() * 2);
        for &s in samples {
            let val = (s * 32767.0).round().clamp(-32768.0, 32767.0) as i16;
            buf.extend_from_slice(&val.to_le_bytes());
        }
        buf
    }

    /// Build a minimal valid SF2 with one preset, one instrument, one sample.
    #[allow(clippy::too_many_arguments)]
    pub fn build_minimal_sf2(
        preset_name: &str,
        inst_name: &str,
        sample_name: &str,
        sample_data: &[f32],
        root_key: u8,
        key_lo: u8,
        key_hi: u8,
        vel_lo: u8,
        vel_hi: u8,
        loop_mode: u16,
        loop_start_offset: u32,
        loop_end_offset: u32,
    ) -> Vec<u8> {
        let num_samples = sample_data.len() as u32;
        let pcm = make_pcm16(sample_data);

        // INFO list (minimal — just ifil version).
        let ifil = {
            let mut d = Vec::new();
            write_u16_le(&mut d, 2); // major
            write_u16_le(&mut d, 1); // minor
            make_chunk(b"ifil", &d)
        };
        let info_list = make_list(b"INFO", &[ifil]);

        // sdta list with smpl sub-chunk.
        let smpl_chunk = make_chunk(b"smpl", &pcm);
        let sdta_list = make_list(b"sdta", &[smpl_chunk]);

        // pdta list.
        // phdr: one preset + EOP terminal
        let mut phdr_buf = Vec::new();
        phdr_buf.extend_from_slice(&make_phdr(preset_name, 0, 0, 0));
        phdr_buf.extend_from_slice(&make_phdr("EOP", 0, 0, 1));
        let phdr_chunk = make_chunk(b"phdr", &phdr_buf);

        // pbag: one preset bag + terminal
        let mut pbag_buf = Vec::new();
        pbag_buf.extend_from_slice(&make_bag(0, 0));
        pbag_buf.extend_from_slice(&make_bag(1, 0)); // terminal
        let pbag_chunk = make_chunk(b"pbag", &pbag_buf);

        // pgen: instrument reference generator
        let mut pgen_buf = Vec::new();
        pgen_buf.extend_from_slice(&make_gen(41, 0)); // GEN_INSTRUMENT -> inst 0
        pgen_buf.extend_from_slice(&make_gen(0, 0)); // terminal
        let pgen_chunk = make_chunk(b"pgen", &pgen_buf);

        // pmod (empty but required by some parsers)
        let pmod_chunk = make_chunk(b"pmod", &[]);

        // inst: one instrument + terminal
        let mut inst_buf = Vec::new();
        inst_buf.extend_from_slice(&make_inst(inst_name, 0));
        inst_buf.extend_from_slice(&make_inst("EOI", 1));
        let inst_chunk = make_chunk(b"inst", &inst_buf);

        // ibag: one instrument bag + terminal
        let mut ibag_buf = Vec::new();
        ibag_buf.extend_from_slice(&make_bag(0, 0));
        ibag_buf.extend_from_slice(&make_bag(4, 0)); // terminal (4 igens)
        let ibag_chunk = make_chunk(b"ibag", &ibag_buf);

        // igen: key range, vel range, sample modes, sampleID
        let mut igen_buf = Vec::new();
        igen_buf.extend_from_slice(&make_gen_range(43, key_lo, key_hi)); // key range
        igen_buf.extend_from_slice(&make_gen_range(44, vel_lo, vel_hi)); // vel range
        igen_buf.extend_from_slice(&make_gen(54, loop_mode as i16)); // sample modes
        igen_buf.extend_from_slice(&make_gen(53, 0)); // sampleID -> shdr 0
        igen_buf.extend_from_slice(&make_gen(0, 0)); // terminal
        let igen_chunk = make_chunk(b"igen", &igen_buf);

        // imod (empty)
        let imod_chunk = make_chunk(b"imod", &[]);

        // shdr: one sample + terminal
        let mut shdr_buf = Vec::new();
        shdr_buf.extend_from_slice(&make_shdr(
            sample_name,
            0,
            num_samples,
            loop_start_offset,
            loop_end_offset,
            44100,
            root_key,
            1, // monoSample
        ));
        shdr_buf.extend_from_slice(&make_shdr("EOS", 0, 0, 0, 0, 0, 0, 0));
        let shdr_chunk = make_chunk(b"shdr", &shdr_buf);

        let pdta_list = make_list(
            b"pdta",
            &[
                phdr_chunk, pbag_chunk, pmod_chunk, pgen_chunk, inst_chunk, ibag_chunk, imod_chunk,
                igen_chunk, shdr_chunk,
            ],
        );

        make_sf2(info_list, sdta_list, pdta_list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_helpers::*;

    #[test]
    fn reject_too_small() {
        assert!(parse_sf2(&[0; 4]).is_err());
    }

    #[test]
    fn reject_non_riff() {
        let mut data = vec![0u8; 12];
        data[0..4].copy_from_slice(b"NOTF");
        assert!(parse_sf2(&data).is_err());
    }

    #[test]
    fn reject_wrong_form_type() {
        let mut data = vec![0u8; 12];
        data[0..4].copy_from_slice(b"RIFF");
        data[4..8].copy_from_slice(&4u32.to_le_bytes());
        data[8..12].copy_from_slice(b"WAVE");
        let err = parse_sf2(&data).unwrap_err();
        assert!(err.contains("sfbk"));
    }

    #[test]
    fn parse_riff_header() {
        // Valid RIFF/sfbk but no lists -> should error about missing chunks
        let mut data = Vec::new();
        write_fourcc(&mut data, b"RIFF");
        write_u32_le(&mut data, 4);
        data.extend_from_slice(b"sfbk");
        let err = parse_sf2(&data).unwrap_err();
        assert!(err.contains("missing"));
    }

    #[test]
    fn parse_minimal_sf2() {
        let sample_data: Vec<f32> = (0..100).map(|i| (i as f32 / 100.0) * 2.0 - 1.0).collect();
        let sf2 = build_minimal_sf2(
            "TestPreset",
            "TestInst",
            "TestSample",
            &sample_data,
            60,  // root key
            36,  // key lo
            84,  // key hi
            0,   // vel lo
            127, // vel hi
            0,   // no loop
            0,   // loop start
            0,   // loop end
        );

        let presets = parse_sf2(&sf2).unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].name, "TestPreset");
        assert_eq!(presets[0].zones.len(), 1);

        let zone = &presets[0].zones[0];
        assert_eq!(zone.name, "TestSample");
        assert_eq!(zone.root_key, 60);
        assert_eq!(zone.key_low, 36);
        assert_eq!(zone.key_high, 84);
        assert_eq!(zone.velocity_low, 0);
        assert_eq!(zone.velocity_high, 127);
        assert_eq!(zone.loop_mode, LoopMode::NoLoop);
        assert_eq!(zone.samples.len(), 100);
        assert_eq!(zone.sample_rate, 44100);
    }

    #[test]
    fn sample_data_conversion() {
        let samples = vec![0.0f32, 0.5, -0.5, 1.0, -1.0];
        let pcm = make_pcm16(&samples);
        let converted = pcm16_to_f32(&pcm, 0, samples.len());
        // Tolerance for 16-bit quantization.
        for (orig, conv) in samples.iter().zip(converted.iter()) {
            assert!((orig - conv).abs() < 0.001, "expected ~{orig}, got {conv}");
        }
    }

    #[test]
    fn forward_loop_mode() {
        let sample_data: Vec<f32> = vec![0.0; 200];
        let sf2 = build_minimal_sf2(
            "LoopPreset",
            "LoopInst",
            "LoopSample",
            &sample_data,
            60,
            0,
            127,
            0,
            127,
            1,   // loop_continuous
            50,  // loop start at sample 50
            150, // loop end at sample 150
        );
        let presets = parse_sf2(&sf2).unwrap();
        let zone = &presets[0].zones[0];
        assert_eq!(zone.loop_mode, LoopMode::Forward);
        assert_eq!(zone.loop_start, Some(50));
        assert_eq!(zone.loop_end, Some(150));
    }

    #[test]
    fn chunk_iteration() {
        // Build two sub-chunks and iterate them.
        let c1 = make_chunk(b"abcd", &[1, 2, 3, 4]);
        let c2 = make_chunk(b"efgh", &[5, 6]);
        let mut combined = Vec::new();
        combined.extend_from_slice(&c1);
        combined.extend_from_slice(&c2);

        let chunks: Vec<_> = iter_chunks(&combined)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(&chunks[0].id, b"abcd");
        assert_eq!(chunks[0].data, &[1, 2, 3, 4]);
        assert_eq!(&chunks[1].id, b"efgh");
        assert_eq!(chunks[1].data, &[5, 6]);
    }

    #[test]
    fn pcm16_empty_range() {
        let result = pcm16_to_f32(&[], 0, 0);
        assert!(result.is_empty());
    }

    #[test]
    fn pcm16_out_of_bounds() {
        let result = pcm16_to_f32(&[0, 0], 0, 100);
        assert!(result.is_empty());
    }

    #[test]
    fn velocity_range_preserved() {
        let sample_data: Vec<f32> = vec![0.0; 50];
        let sf2 = build_minimal_sf2(
            "VelPreset",
            "VelInst",
            "VelSample",
            &sample_data,
            60,
            0,
            127,
            32,
            96,
            0,
            0,
            0,
        );
        let presets = parse_sf2(&sf2).unwrap();
        let zone = &presets[0].zones[0];
        assert_eq!(zone.velocity_low, 32);
        assert_eq!(zone.velocity_high, 96);
    }

    #[test]
    fn shdr_record_parsing() {
        let rec = make_shdr("MySample", 100, 200, 110, 190, 48000, 64, 1);
        let records = parse_shdr_records(&rec).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].name, "MySample");
        assert_eq!(records[0].start, 100);
        assert_eq!(records[0].end, 200);
        assert_eq!(records[0].loop_start, 110);
        assert_eq!(records[0].loop_end, 190);
        assert_eq!(records[0].sample_rate, 48000);
        assert_eq!(records[0].original_pitch, 64);
    }

    /// Regression: loop_start < start in a malformed SF2 should saturate to
    /// 0 instead of wrapping around via integer underflow.
    #[test]
    fn loop_points_before_sample_start_no_underflow() {
        let sample_data: Vec<f32> = vec![0.0; 200];
        // loop_start=5 < start=0 is fine, but test the subtraction path
        // by building a sample where loop_start offset < sample start
        // offset — which would cause underflow without saturating_sub.
        let sf2 = build_minimal_sf2(
            "UnderflowPreset",
            "UnderflowInst",
            "UnderflowSample",
            &sample_data,
            60,
            0,
            127,
            0,
            127,
            1,  // loop_continuous
            0,  // loop_start (== start, so offset is 0)
            50, // loop_end
        );
        let presets = parse_sf2(&sf2).unwrap();
        let zone = &presets[0].zones[0];
        // loop_start should be 0 (0 - 0 = 0), loop_end should be 50 (50 - 0 = 50)
        assert_eq!(zone.loop_start, Some(0));
        assert_eq!(zone.loop_end, Some(50));
    }
}
