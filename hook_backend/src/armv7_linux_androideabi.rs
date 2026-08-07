use std::os::raw::c_int;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering};

use crate::{HookName, Priority, UninstallError};

extern "C" {
    fn registerInlineHook(target_addr: u32, new_addr: u32, proto_addr: *mut *mut u32) -> c_int;
    fn inlineHook(target_addr: u32) -> c_int;
    fn inlineUnHook(target_addr: u32) -> c_int;
}

/// A function hook specific to `ARMv7` Android
///
/// Only a single hook may be installed per target; `name` and `priority`
/// are accepted for API parity with other backends but otherwise ignored
#[derive(Debug)]
pub struct Hook {
    target: AtomicPtr<()>,
    original: AtomicPtr<u32>,
}

impl Hook {
    /// Creates a new, unitialized hook
    pub const fn new() -> Self {
        Self {
            target: AtomicPtr::new(null_mut()),
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
        let target_addr = target as u32;
        let hook_addr = hook as u32;
        let mut original: *mut u32 = null_mut();

        if registerInlineHook(target_addr, hook_addr, &mut original) != 0
            || inlineHook(target_addr) != 0
        {
            return false;
        }

        self.target.store(target as *mut (), Ordering::SeqCst);
        self.original.store(original, Ordering::SeqCst);
        true
    }

    /// Uninstalls the hook
    ///
    /// # Safety
    /// No other thread may be currently executing inside the hook or
    /// original functions in a way that assumes this hook remains installed
    pub unsafe fn uninstall(&self) -> Result<(), UninstallError> {
        let target = self.target.load(Ordering::SeqCst);
        if target.is_null() {
            return Err(UninstallError::NotInstalled);
        }
        if inlineUnHook(target as u32) != 0 {
            return Err(UninstallError::Failed);
        }
        self.target.store(null_mut(), Ordering::SeqCst);
        self.original.store(null_mut(), Ordering::SeqCst);
        Ok(())
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
