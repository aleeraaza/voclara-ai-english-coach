//! Delivery of one-time codes. A WhatsApp/SMS implementation is added once a
//! provider is chosen; until then `LogOtpSender` prints codes (dev only).

use async_trait::async_trait;

/// `async_trait` lets us store the sender as `Arc<dyn OtpSender>`; plain
/// `async fn` in traits is not yet usable through `dyn`.
#[async_trait]
pub trait OtpSender: Send + Sync {
    async fn send(&self, phone: &str, code: &str) -> anyhow::Result<()>;
}

/// Development sender: writes the code to the log instead of sending it.
/// `Config::validate` refuses this sender in production.
pub struct LogOtpSender;

#[async_trait]
impl OtpSender for LogOtpSender {
    async fn send(&self, phone: &str, code: &str) -> anyhow::Result<()> {
        tracing::warn!(%phone, %code, "DEV ONLY: one-time code (no SMS/WhatsApp provider configured)");
        Ok(())
    }
}
