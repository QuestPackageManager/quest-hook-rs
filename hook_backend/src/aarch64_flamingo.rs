use std::ffi::c_void;
use std::sync::Mutex;

use flamingo_rs::{
    HookBuilder, HookFilter as NativeHookFilter, HookName as NativeHookName, InstalledHook,
    Instruction, UninstallOutcome,
};

use crate::{HookFilter, HookName, Priority, UninstallError};

/// A function hook specific to `ARMv8` Android, supporting multiple hooks
/// per target ordered by [`Priority`]
#[derive(Debug, Default)]
pub struct FunctionHook {
    installed: Mutex<Option<InstalledHook>>,
}

impl FunctionHook {
    /// Creates a new, unitialized hook
    pub const fn new() -> Self {
        Self {
            installed: Mutex::new(None),
        }
    }

    /// Installes the hook by redirecting `target` to `hook`, returning true
    /// on success
    ///
    /// `name` and `priority` are forwarded to the native library, which
    /// enforces `priority` against any other hooks already installed at
    /// `target`
    ///
    /// # Safety
    /// `target` and `hook` must have the same signature and calling
    /// convention
    pub unsafe fn install(
        &self,
        target: *const (),
        hook: *const (),
        name: HookName,
        priority: Priority,
    ) -> bool {
        let Ok(name_info) = NativeHookName::namespaced(name.namespace, name.name) else {
            return false;
        };

        let mut builder = HookBuilder::new().name(name_info);
        for filter in priority.before {
            let Ok(filter) = to_native_filter(filter) else {
                return false;
            };
            builder = builder.before(filter);
        }
        for filter in priority.after {
            let Ok(filter) = to_native_filter(filter) else {
                return false;
            };
            builder = builder.after(filter);
        }

        let installed = unsafe { builder.install(target as *mut Instruction, hook as *mut c_void) };
        match installed {
            Ok(installed) => {
                *self.installed.lock().unwrap() = Some(installed);
                true
            }
            Err(_) => false,
        }
    }

    /// Uninstalls the hook. Other hooks that share the same target are
    /// unaffected
    ///
    /// # Safety
    /// No other thread may be currently executing inside the hook or
    /// original functions in a way that assumes this hook remains installed
    pub unsafe fn uninstall(&self) -> Result<(), UninstallError> {
        let Some(installed) = self.installed.lock().unwrap().take() else {
            return Err(UninstallError::NotInstalled);
        };
        match installed.uninstall() {
            UninstallOutcome::Removed { .. } => Ok(()),
            UninstallOutcome::RemapFailure => Err(UninstallError::Failed),
        }
    }

    /// Whether the hook is installed
    pub fn is_installed(&self) -> bool {
        self.installed.lock().unwrap().is_some()
    }

    /// Returns the address of a trampoline function to the original target,
    /// if installed
    pub fn original(&self) -> Option<*const ()> {
        self.installed.lock().unwrap().as_ref()?.original()
    }
}

fn to_native_filter(filter: HookFilter) -> Result<NativeHookFilter, std::ffi::NulError> {
    NativeHookFilter::new(filter.namespace, filter.name)
}
