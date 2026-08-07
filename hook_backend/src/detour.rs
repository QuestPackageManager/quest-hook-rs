use std::sync::OnceLock;

use retour::RawDetour;

use crate::{HookName, Priority, UninstallError};

/// A function hook that works across most platforms
///
/// Only a single hook may be installed per target; `name` and `priority`
/// are accepted for API parity with other backends but otherwise ignored
#[derive(Debug)]
pub struct Hook {
    detour: OnceLock<RawDetour>,
}

impl Hook {
    /// Creates a new, unitialized hook
    pub const fn new() -> Self {
        Self {
            detour: OnceLock::new(),
        }
    }

    /// Installes the hook by redirecting `target` to `hook`, returning true on
    /// success
    ///
    /// # Safety
    /// `target` and `hook` must have the same signature and calling convention
    pub unsafe fn install(
        &self,
        target: *const (),
        hook: *const (),
        _name: HookName,
        _priority: Priority,
    ) -> bool {
        match RawDetour::new(target, hook) {
            Ok(detour) if detour.enable().is_ok() => {
                self.detour.set(detour).ok();
                true
            }
            _ => false,
        }
    }

    /// Uninstalls the hook
    ///
    /// # Safety
    /// No other thread may be currently executing inside the hook or
    /// original functions in a way that assumes this hook remains installed
    pub unsafe fn uninstall(&self) -> Result<(), UninstallError> {
        match self.detour.get() {
            Some(detour) if detour.is_enabled() => {
                detour.disable().map_err(|_| UninstallError::Failed)
            }
            _ => Err(UninstallError::NotInstalled),
        }
    }

    /// Whether the hook is installed
    pub fn is_installed(&self) -> bool {
        self.detour.get().is_some_and(RawDetour::is_enabled)
    }

    /// Returns the address of a trampoline function to the original target, if
    /// installed
    pub fn original(&self) -> Option<*const ()> {
        self.detour.get().map(|d| d.trampoline() as *const ())
    }
}

impl Default for Hook {
    fn default() -> Self {
        Self::new()
    }
}
