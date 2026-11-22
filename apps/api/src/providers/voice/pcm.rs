//! PCM16 helpers for relay codecs (e.g. OpenAI wants 24 kHz, the app sends 16 kHz).

use base64::{Engine, engine::general_purpose::STANDARD};

use crate::error::{AppError, AppResult};

/// Base64 PCM16 little-endian -> samples. A trailing odd byte is ignored.
pub fn decode_pcm16(b64: &str) -> AppResult<Vec<i16>> {
    let bytes = STANDARD
        .decode(b64.trim())
        .map_err(|_| AppError::BadRequest("audio is not valid base64".into()))?;
    // `as_chunks::<2>()` yields `[u8; 2]` arrays (the odd byte, if any, is in
    // the ignored remainder), so no indexing is needed.
    let (pairs, _rest) = bytes.as_chunks::<2>();
    Ok(pairs.iter().map(|&b| i16::from_le_bytes(b)).collect())
}

pub fn encode_pcm16(samples: &[i16]) -> String {
    let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    STANDARD.encode(bytes)
}

/// Linear-interpolation resampler; good enough for speech at these ratios.
pub fn resample(input: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    if from_rate == to_rate || input.is_empty() {
        return input.to_vec();
    }
    let out_len =
        ((input.len() as u64 * u64::from(to_rate)) / u64::from(from_rate)).max(1) as usize;
    let step = f64::from(from_rate) / f64::from(to_rate);
    let last = input.len() - 1;
    (0..out_len)
        .map(|i| {
            let pos = i as f64 * step;
            let i0 = (pos.floor() as usize).min(last);
            let i1 = (i0 + 1).min(last);
            let frac = pos - i0 as f64;
            let v = f64::from(input[i0]) * (1.0 - frac) + f64::from(input[i1]) * frac;
            v.round().clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let s = vec![0, 1, -1, i16::MAX, i16::MIN];
        assert_eq!(decode_pcm16(&encode_pcm16(&s)).unwrap(), s);
    }

    #[test]
    fn rejects_bad_base64() {
        assert!(decode_pcm16("not base64!").is_err());
    }

    #[test]
    fn resample_16k_to_24k_scales_length_and_keeps_level() {
        let out = resample(&vec![1000; 1600], 16_000, 24_000);
        assert_eq!(out.len(), 2400);
        assert!(out.iter().all(|&v| v == 1000));
        assert_eq!(resample(&[5, 6], 24_000, 24_000), vec![5, 6]);
    }
}
