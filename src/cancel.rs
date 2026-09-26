//! Cancelling a running operation. Escape in the main window, or Cancel while
//! waiting for sign-in, sets it; long-running work checks it between steps and
//! awaits it alongside anything that could wait for a long time.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::Notify;

#[derive(Clone, Default)]
pub struct Cancel(Arc<Inner>);

#[derive(Default)]
struct Inner {
    flag: AtomicBool,
    notify: Notify,
}

impl Cancel {
    pub fn new() -> Cancel {
        Cancel::default()
    }

    pub fn cancel(&self) {
        self.0.flag.store(true, Ordering::SeqCst);
        self.0.notify.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.flag.load(Ordering::SeqCst)
    }

    /// Resolves once cancelled.
    pub async fn cancelled(&self) {
        loop {
            let notified = self.0.notify.notified();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }

    /// `Err(Cancelled)` if cancelled, for use with `?` between steps.
    pub fn check(&self) -> crate::error::Result<()> {
        if self.is_cancelled() { Err(crate::error::Error::Cancelled) } else { Ok(()) }
    }
}
