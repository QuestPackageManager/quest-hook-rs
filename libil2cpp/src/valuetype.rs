use crate::{Arguments, Returned, Type};

/// Extension trait for value types providing additional functionality
pub trait ValueTypeExt: for<'a> Type<Held<'a> = Self> + Sized {
    /// Invokes the method with the given name on `self` using the given
    /// arguments, with type checking
    ///
    /// # Panics
    ///
    /// This method will panic if a matching method can't be found.
    fn invoke<A, R, const N: usize>(&mut self, name: &str, args: A) -> crate::Result<R>
    where
        A: Arguments<N>,
        R: Returned,
    {
        let method = Self::class().find_method::<A, R, N>(name).unwrap();
        unsafe { method.invoke_unchecked(self, args) }
    }

    /// Invokes the `void` method with the given name on `self` using the
    /// given arguments, with type checking
    ///
    /// # Panics
    ///
    /// This method will panic if a matching method can't be found.
    fn invoke_void<A, const N: usize>(&mut self, name: &str, args: A) -> crate::Result<()>
    where
        A: Arguments<N>,
    {
        let method = Self::class().find_method::<A, (), N>(name).unwrap();
        unsafe { method.invoke_unchecked(self, args) }
    }
}

impl<T> ValueTypeExt for T where T: for<'a> Type<Held<'a> = T> {}

/// Padding type for value types
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueTypePadding<const N: usize>(pub [u8; N]);

impl<const N: usize> Default for ValueTypePadding<N> {
    fn default() -> Self {
        Self([0; N])
    }
}
