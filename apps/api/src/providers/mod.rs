//! External services, each behind a trait so tests can swap in fakes.
//! Pronunciation and TTS providers join here in later steps.

pub mod google;
pub mod grader;
pub mod otp;
pub mod voice;
