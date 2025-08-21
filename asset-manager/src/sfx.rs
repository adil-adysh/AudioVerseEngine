use crate::util::{AssetError, MAX_SFX_FRAMES};

#[derive(Debug, Clone)]
pub enum SampleFormat {
    F32,
    S16,
    U8,
}

#[derive(Debug, Clone)]
pub struct SfxBlob {
    pub samples: Vec<f32>, // interleaved f32
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: u64,
    pub loop_points: Option<(u64, u64)>,
}

impl SfxBlob {
    pub fn from_sfx_bytes(bytes: &[u8]) -> Result<Self, AssetError> {
        if bytes.len() < 20 {
            return Err(AssetError::Decode("sfx too small".into()));
        }
        let magic = &bytes[0..4];
        if magic != b"SFX1" {
            return Err(AssetError::Decode("bad sfx magic".into()));
        }
        let sf = bytes[4];
        let channels = bytes[5] as u16;
        let sample_rate = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let frames = u64::from_le_bytes([
            bytes[12], bytes[13], bytes[14], bytes[15], bytes[16], bytes[17], bytes[18], bytes[19]
        ]);
        if frames == 0 || frames > MAX_SFX_FRAMES {
            return Err(AssetError::ResourceLimit("sfx frame count unreasonable".into()));
        }

        let sample_format = match sf {
            0 => SampleFormat::F32,
            1 => SampleFormat::S16,
            2 => SampleFormat::U8,
            _ => return Err(AssetError::Decode("unknown sample format".into())),
        };

        let bytes_per_sample = match sample_format {
            SampleFormat::F32 => 4usize,
            SampleFormat::S16 => 2usize,
            SampleFormat::U8 => 1usize,
        };

        let expected = (frames as usize)
            .checked_mul(channels as usize)
            .and_then(|n| n.checked_mul(bytes_per_sample))
            .ok_or_else(|| AssetError::ResourceLimit("overflow computing sfx size".into()))?;
        if bytes.len() < 20 + expected {
            return Err(AssetError::Decode("file truncated".into()));
        }

        let mut samples = Vec::with_capacity((frames as usize) * channels as usize);
        let mut idx = 20usize;
        match sample_format {
            SampleFormat::F32 => {
                while idx + 4 <= 20 + expected {
                    let mut b = [0u8;4];
                    b.copy_from_slice(&bytes[idx..idx+4]);
                    samples.push(f32::from_le_bytes(b));
                    idx += 4;
                }
            }
            SampleFormat::S16 => {
                while idx + 2 <= 20 + expected {
                    let mut b = [0u8;2];
                    b.copy_from_slice(&bytes[idx..idx+2]);
                    let v = i16::from_le_bytes(b) as f32 / i16::MAX as f32;
                    samples.push(v);
                    idx += 2;
                }
            }
            SampleFormat::U8 => {
                while idx < 20 + expected {
                    let v = bytes[idx] as f32 / 255.0 * 2.0 - 1.0;
                    samples.push(v);
                    idx += 1;
                }
            }
        }

        Ok(SfxBlob {
            samples,
            sample_rate,
            channels,
            frames,
            loop_points: None,
        })
    }
}
