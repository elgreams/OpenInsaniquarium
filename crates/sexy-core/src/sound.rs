//! Sound file helpers. The original plays `.ogg` and Sun `.au` (8-bit μ-law,
//! ~8 kHz mono) via DirectSound/BASS.

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SoundError {
    #[error("not a Sun .au file")]
    BadMagic,
    #[error("truncated .au file")]
    Truncated,
    #[error("unsupported .au encoding {0}")]
    Encoding(u32),
}

/// Decoded PCM.
#[derive(Debug, Clone, PartialEq)]
pub struct Pcm {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<i16>,
}

/// Decode a Sun/NeXT `.au` file. Supports μ-law (1) and 16-bit linear (3),
/// which covers the shipped data (all μ-law).
pub fn decode_au(b: &[u8]) -> Result<Pcm, SoundError> {
    if b.get(..4) != Some(b".snd") {
        return Err(SoundError::BadMagic);
    }
    let word = |i: usize| -> Result<u32, SoundError> {
        b.get(i..i + 4)
            .map(|s| u32::from_be_bytes(s.try_into().unwrap()))
            .ok_or(SoundError::Truncated)
    };
    let (offset, size, enc, rate, channels) =
        (word(4)? as usize, word(8)?, word(12)?, word(16)?, word(20)?);
    let data = b.get(offset..).ok_or(SoundError::Truncated)?;
    // 0xFFFFFFFF means "unknown size": use the rest of the file.
    let data = match size {
        u32::MAX => data,
        n => &data[..data.len().min(n as usize)],
    };
    let samples = match enc {
        1 => data.iter().map(|&x| ulaw_to_linear(x)).collect(),
        3 => data
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&c| i16::from_be_bytes(c))
            .collect(),
        e => return Err(SoundError::Encoding(e)),
    };
    Ok(Pcm {
        sample_rate: rate,
        channels: channels as u16,
        samples,
    })
}

/// ITU-T G.711 μ-law expansion.
fn ulaw_to_linear(u: u8) -> i16 {
    let u = !u;
    let sign = u & 0x80;
    let exponent = (u >> 4) & 0x07;
    let mantissa = u & 0x0F;
    let magnitude = ((((mantissa as i32) << 3) + 0x84) << exponent) - 0x84;
    (if sign != 0 { -magnitude } else { magnitude }) as i16
}

impl Pcm {
    /// Encode as a 16-bit PCM RIFF/WAVE file.
    pub fn to_wav(&self) -> Vec<u8> {
        let data_len = (self.samples.len() * 2) as u32;
        let block_align = self.channels * 2;
        let mut w = Vec::with_capacity(44 + data_len as usize);
        w.extend_from_slice(b"RIFF");
        w.extend_from_slice(&(36 + data_len).to_le_bytes());
        w.extend_from_slice(b"WAVEfmt ");
        w.extend_from_slice(&16u32.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes());
        w.extend_from_slice(&self.channels.to_le_bytes());
        w.extend_from_slice(&self.sample_rate.to_le_bytes());
        w.extend_from_slice(&(self.sample_rate * block_align as u32).to_le_bytes());
        w.extend_from_slice(&block_align.to_le_bytes());
        w.extend_from_slice(&16u16.to_le_bytes());
        w.extend_from_slice(b"data");
        w.extend_from_slice(&data_len.to_le_bytes());
        for s in &self.samples {
            w.extend_from_slice(&s.to_le_bytes());
        }
        w
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ulaw_reference_points() {
        assert_eq!(ulaw_to_linear(0xFF), 0);
        assert_eq!(ulaw_to_linear(0x7F), 0);
        assert_eq!(ulaw_to_linear(0x80), 32124);
        assert_eq!(ulaw_to_linear(0x00), -32124);
    }

    #[test]
    fn decodes_minimal_au() {
        let mut f = b".snd".to_vec();
        for v in [24u32, 2, 1, 8000, 1] {
            f.extend_from_slice(&v.to_be_bytes());
        }
        f.extend_from_slice(&[0xFF, 0x80]);
        let p = decode_au(&f).unwrap();
        assert_eq!(p.samples, [0, 32124]);
        let wav = p.to_wav();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(wav.len(), 44 + 4);
    }

    #[test]
    fn decodes_shipped_au_files() {
        let Some(d) = crate::data_dir::test_data_dir() else {
            return;
        };
        for e in std::fs::read_dir(d.join("sounds")).unwrap().flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "au") {
                let pcm = decode_au(&std::fs::read(&p).unwrap()).unwrap();
                assert!(pcm.sample_rate >= 8000 && !pcm.samples.is_empty());
            }
        }
    }
}
