use std::ffi::c_void;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering};

use crate::{HookName, Priority, UninstallError};

extern "C" {
    fn A64HookFunction(symbol: *const c_void, replace: *const c_void, result: *mut *mut c_void);
}

/// A function hook specific to `ARMv8` Android
///
/// Only a single hook may be installed per target; `name` and `priority`
/// are accepted for API parity with other backends but otherwise ignored.
/// Prefer the `flamingo` backend if multiple hooks need to share a target
#[derive(Debug)]
pub struct Hook {
    original: AtomicPtr<c_void>,
}

impl Hook {
    /// Creates a new, unitialized hook
    pub const fn new() -> Self {
        Self {
            original: AtomicPtr::new(null_mut()),
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
        let mut original: *mut c_void = null_mut();

        A64HookFunction(target.cast(), hook.cast(), &mut original);

        self.original.store(original, Ordering::SeqCst);
        true
    }

    /// Always fails with [`UninstallError::Unsupported`]: the underlying
    /// `And64InlineHook` library exposes no way to remove an installed
    /// hook. Prefer the `flamingo` backend if runtime uninstallation is
    /// required
    ///
    /// # Safety
    /// See [`Self::install`]
    pub unsafe fn uninstall(&self) -> Result<(), UninstallError> {
        Err(UninstallError::Unsupported)
    }

    /// Whether the hook is installed
    pub fn is_installed(&self) -> bool {
        !self.original.load(Ordering::SeqCst).is_null()
    }

    /// Returns the address of a trampoline function to the original target, if
    /// installed
    pub fn original(&self) -> Option<*const ()> {
        match self.original.load(Ordering::SeqCst) {
            null if null.is_null() => None,
            original => Some(original as *const ()),
        }
    }
}
