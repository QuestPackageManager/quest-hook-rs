use std::ffi::c_void;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering};

use flamingo_rs::HookBuilder;

/// A function hook specific to `ARMv8` Android
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
    pub unsafe fn install(&self, target: *const (), hook: *const ()) -> bool {
        let installed =
            unsafe { HookBuilder::new().install(target as *mut u32, hook as *mut c_void) };

        let Ok(installed) = installed else {
            return false;
        };

        let original = installed.original().map_or(null_mut(), |p| p as *mut c_void);
        self.original.store(original, Ordering::SeqCst);

        // The hook is meant to live for the rest of the process, so leak the
        // handle instead of uninstalling it when it goes out of scope.
        std::mem::forget(installed);

        true
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
