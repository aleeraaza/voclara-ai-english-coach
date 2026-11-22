//! One-time code helpers: phone normalisation, code generation and hashing.

use rand::Rng;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::error::{AppError, AppResult};

pub const CODE_TTL_SECS: i64 = 300;
pub const RESEND_AFTER_SECS: i64 = 30;
pub const MAX_SENDS_PER_HOUR: i64 = 5;
pub const MAX_ATTEMPTS: i32 = 5;

/// Accept E.164 numbers, tolerating spaces, dashes and brackets:
/// "+92 300-123 4567" -> "+923001234567".
pub fn normalize_phone(raw: &str) -> AppResult<String> {
    let cleaned: String = raw
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | '(' | ')' | '.'))
        .collect();
    let digits = cleaned.strip_prefix('+').ok_or_else(|| {
        AppError::BadRequest("phone must be in international format, e.g. +923001234567".into())
    })?;
    let valid = (8..=15).contains(&digits.len())
        && digits.chars().all(|c| c.is_ascii_digit())
        && !digits.starts_with('0');
    if !valid {
        return Err(AppError::BadRequest("phone number is not valid".into()));
    }
    Ok(cleaned)
}

/// A uniformly random 6-digit code, zero-padded.
pub fn generate_code() -> String {
    format!("{:06}", rand::rng().random_range(0..1_000_000u32))
}

/// Keyed hash so a database leak alone does not reveal live codes.
pub fn hash_code(secret: &[u8], phone: &str, code: &str) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(secret);
    h.update([0u8]);
    h.update(phone.as_bytes());
    h.update([0u8]);
    h.update(code.trim().as_bytes());
    h.finalize().to_vec()
}

/// Constant-time comparison, so response timing does not leak how close a guess was.
pub fn code_matches(expected_hash: &[u8], secret: &[u8], phone: &str, code: &str) -> bool {
    hash_code(secret, phone, code).ct_eq(expected_hash).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_common_formats() {
        assert_eq!(
            normalize_phone("+92 300-123 4567").unwrap(),
            "+923001234567"
        );
        assert_eq!(
            normalize_phone("+971 (50) 123 4567").unwrap(),
            "+971501234567"
        );
    }

    #[test]
    fn rejects_bad_numbers() {
        for bad in [
            "03001234567",
            "+0300123456",
            "+92abc",
            "+123",
            "+1234567890123456",
        ] {
            assert!(normalize_phone(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn codes_are_six_digits() {
        for _ in 0..200 {
            let c = generate_code();
            assert_eq!(c.len(), 6);
            assert!(c.chars().all(|ch| ch.is_ascii_digit()));
        }
    }

    #[test]
    fn hash_binds_phone_and_code() {
        let s = b"secret";
        let h = hash_code(s, "+923001234567", "123456");
        assert!(code_matches(&h, s, "+923001234567", "123456"));
        assert!(code_matches(&h, s, "+923001234567", " 123456 "));
        assert!(!code_matches(&h, s, "+923001234567", "123457"));
        assert!(!code_matches(&h, s, "+923001234568", "123456"));
        assert!(!code_matches(&h, b"other", "+923001234567", "123456"));
    }
}
