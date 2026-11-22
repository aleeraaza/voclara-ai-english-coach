//! External services, each behind a trait so tests can swap in fakes.
//! Grader, pronunciation and TTS providers join here in later steps.

pub mod google;
pub mod otp;
pub mod voice;
