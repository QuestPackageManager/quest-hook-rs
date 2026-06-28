use std::fmt;

use crate::{raw, Il2CppType, WrapRaw};

/// Information about a C# parameter
#[repr(transparent)]
pub struct ParameterInfo(&'static Il2CppType);

unsafe impl Send for ParameterInfo {}
unsafe impl Sync for ParameterInfo {}

impl ParameterInfo {
    /// Type of the parameter
    pub fn ty(&self) -> &Il2CppType {
        self.0
    }
}

impl fmt::Debug for ParameterInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParameterInfo")
            .field("ty", &self.ty())
            .finish()
    }
}

impl fmt::Display for ParameterInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ty())
    }
}

unsafe impl WrapRaw for ParameterInfo {
    type Raw = raw::Il2CppType;
}
