use std::ffi::CStr;
use std::fmt::{self, Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};

use crate::{raw, Gc, Il2CppObject, Il2CppString, RefType, WrapRaw};

/// An il2cpp exception
#[repr(transparent)]
pub struct Il2CppException(raw::Il2CppException);

impl Il2CppException {
    /// Exception message
    pub fn message(&self) -> Option<&Il2CppString> {
        unsafe { Il2CppString::wrap_ptr(self.raw().message) }
    }

    /// Inner exception
    pub fn inner_exception(&self) -> Option<&Self> {
        unsafe { Self::wrap_ptr(self.raw().inner_ex) }
    }

    /// Iterator over the inner exceptions, starting with the exception itself
    pub fn trace(&self) -> Trace<'_> {
        Trace {
            current: Some(self),
        }
    }

    /// Exception source
    pub fn source(&self) -> Option<&Il2CppString> {
        unsafe { Il2CppString::wrap_ptr(self.raw().source) }
    }

    /// Throws the exception
    ///
    /// # Safety
    /// This is implemented as a C++ throw, which is UB when called from Rust.
    /// Therefore this method is UB, and only provided just in case ™️. (in
    /// simpler terms, this method is never safe)
    pub unsafe fn throw(&self) -> ! {
        raw::raise_exception(self.raw())
    }
}

/// Iterator over inner exceptions
#[derive(Debug)]
pub struct Trace<'a> {
    current: Option<&'a Il2CppException>,
}

unsafe impl WrapRaw for Il2CppException {
    type Raw = raw::Il2CppException;
}

impl<'a> Iterator for Trace<'a> {
    type Item = &'a Il2CppException;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current {
            Some(e) => {
                self.current = e.inner_exception();
                Some(e)
            }
            None => None,
        }
    }
}

impl AsRef<Il2CppObject> for Il2CppException {
    fn as_ref(&self) -> &Il2CppObject {
        unsafe { Il2CppObject::wrap(&self.raw().object) }
    }
}

impl AsMut<Il2CppObject> for Il2CppException {
    fn as_mut(&mut self) -> &mut Il2CppObject {
        unsafe { Il2CppObject::wrap_mut(&mut self.raw_mut().object) }
    }
}

impl Deref for Il2CppException {
    type Target = Il2CppObject;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl DerefMut for Il2CppException {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

// Uses `il2cpp_format_exception` (message + full stack trace, matching C#'s
// own `Exception.ToString()`) rather than hand-formatting `class: message`,
// for parity with beatsaber-hook's `exception_to_string`.
// <https://github.com/QuestPackageManager/beatsaber-hook/blob/7632eb7bf2634dabbf3cade1df140e5d93f48845/src/exceptions.cpp#L9-L13>
impl fmt::Display for Il2CppException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const BUF_SIZE: usize = 4096;
        let mut buf = [0u8; BUF_SIZE];
        unsafe { raw::format_exception(self.raw(), buf.as_mut_ptr().cast(), BUF_SIZE as i32) };
        let message = unsafe { CStr::from_ptr(buf.as_ptr().cast()) };
        f.write_str(&message.to_string_lossy())
    }
}

impl fmt::Debug for Il2CppException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Il2CppException")
            .field("class", self.as_object().class())
            .field("message", &self.message())
            .field("source", &self.source())
            .finish()
    }
}

impl std::error::Error for Il2CppException {}
impl std::error::Error for &mut Il2CppException {}
impl std::error::Error for Gc<Il2CppException> {}

impl Display for Gc<Il2CppException> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&**self, f)
    }
}
