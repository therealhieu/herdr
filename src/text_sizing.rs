//! Kitty text sizing protocol (OSC 66) support.
//!
//! Sized text is modeled outside the terminal core: each block is written into
//! the core grid as self-describing placeholder cells (see [`cell`]), so it
//! inherits scrolling, reflow, erasing, and persistence from the core. Clients
//! that run on a host able to size text re-emit OSC 66 for intact blocks.
//!
//! The server only synthesizes blocks while at least one attached client can
//! draw them; otherwise OSC 66 stays a no-op, as before.

pub(crate) mod cell;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

/// Whether the pane byte stream turns OSC 66 into placeholder blocks.
static ACTIVE: AtomicBool = AtomicBool::new(false);
/// Whether this client process runs on a host terminal that can size text.
static HOST_SUPPORTED: AtomicBool = AtomicBool::new(false);
static REGISTRY: Mutex<RegistryState> = Mutex::new(RegistryState::new());

#[cfg(test)]
thread_local! {
    static ACTIVE_OVERRIDE: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// Returns whether OSC 66 sequences in pane output create sized blocks.
pub(crate) fn is_active() -> bool {
    #[cfg(test)]
    if let Some(active) = ACTIVE_OVERRIDE.with(std::cell::Cell::get) {
        return active;
    }
    ACTIVE.load(Ordering::Relaxed)
}

/// Overrides [`is_active`] for the current test thread until the guard drops.
#[cfg(test)]
pub(crate) fn set_active_for_test(active: bool) -> ActiveOverrideGuard {
    let previous = ACTIVE_OVERRIDE.with(|cell| cell.replace(Some(active)));
    ActiveOverrideGuard { previous }
}

#[cfg(test)]
pub(crate) struct ActiveOverrideGuard {
    previous: Option<bool>,
}

#[cfg(test)]
impl Drop for ActiveOverrideGuard {
    fn drop(&mut self) {
        ACTIVE_OVERRIDE.with(|cell| cell.set(self.previous));
    }
}

/// Records whether this client's host terminal can draw sized text.
// Wired by client capability negotiation; unused until then.
#[allow(dead_code)]
pub(crate) fn set_host_supported(supported: bool) {
    HOST_SUPPORTED.store(supported, Ordering::Relaxed);
}

/// Returns whether this client's host terminal can draw sized text.
// Read by client presentation; unused until then.
#[allow(dead_code)]
pub(crate) fn host_supported() -> bool {
    HOST_SUPPORTED.load(Ordering::Relaxed)
}

/// Server-wide count of attached clients and how many can size text.
///
/// Sizing is active while at least one attached client supports it. When the
/// last client detaches, the previous value is kept so detach and reattach do
/// not change how pane output is interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RegistryState {
    supporting: usize,
    attached: usize,
    active: bool,
}

// Exercised once client connections register; unused until then.
#[allow(dead_code)]
impl RegistryState {
    pub(crate) const fn new() -> Self {
        Self {
            supporting: 0,
            attached: 0,
            active: false,
        }
    }

    pub(crate) fn register(&mut self, supporting: bool) {
        self.attached += 1;
        if supporting {
            self.supporting += 1;
        }
        self.recompute();
    }

    pub(crate) fn unregister(&mut self, supporting: bool) {
        self.attached = self.attached.saturating_sub(1);
        if supporting {
            self.supporting = self.supporting.saturating_sub(1);
        }
        self.recompute();
    }

    pub(crate) fn active(&self) -> bool {
        self.active
    }

    fn recompute(&mut self) {
        if self.attached > 0 {
            self.active = self.supporting > 0;
        }
    }
}

/// Registers one attached client for its connection lifetime.
// Held by client connections once capability negotiation lands.
#[allow(dead_code)]
pub(crate) struct ClientRegistration {
    supporting: bool,
}

// Constructed by client connections once capability negotiation lands.
#[allow(dead_code)]
impl ClientRegistration {
    pub(crate) fn new(supporting: bool) -> Self {
        update_registry(|registry| registry.register(supporting));
        Self { supporting }
    }
}

impl Drop for ClientRegistration {
    fn drop(&mut self) {
        let supporting = self.supporting;
        update_registry(|registry| registry.unregister(supporting));
    }
}

fn update_registry(change: impl FnOnce(&mut RegistryState)) {
    let mut registry = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    change(&mut registry);
    ACTIVE.store(registry.active(), Ordering::Relaxed);
}
