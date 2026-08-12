//! Raw il2cpp types and functions
//!
//! This module contains raw C types defined in libil2cpp and raw C functions
//! dynamically loaded from libil2cpp.so.

mod functions;
mod gc;

#[cfg(not(feature = "bindgen"))]
#[cfg_attr(feature = "il2cpp_v31", path = "types_v31/mod.rs")]
#[cfg_attr(feature = "il2cpp_v29", path = "types_v29.rs")]
#[cfg_attr(feature = "il2cpp_v24", path = "types_v24.rs")]
#[cfg_attr(feature = "unity2018", path = "types_2018.rs")]
mod types;

#[cfg(feature = "bindgen")]
#[path = "bindgen.rs"]
mod types;

pub use functions::*;
pub use gc::*;
pub use types::*;

use std::{
    ffi::c_void,
    mem::{size_of, transmute},
};

use crate::{BoxedValue, Gc, ValueType};

/// Safe wrapper around a raw il2cpp type which can be used in its place
///
/// # Safety
/// The wrapper must have the exact same representation as the underlying raw
/// il2cpp type, which means it has to be `#[repr(transparent)]`.
pub unsafe trait WrapRaw: Sized {
    /// Raw il2cpp type
    type Raw;

    /// Returns a reference to the underlying raw il2cpp type
    #[inline]
    fn raw(&self) -> &Self::Raw {
        unsafe { &*(self as *const Self).cast() }
    }

    /// Returns a mutable reference to the underlying raw il2cpp type
    ///
    /// # Safety
    /// This method is unsafe because it allows mutating the underlying type in
    /// ways that make it invalid. Avoid mutating raw il2cpp types unless you
    /// know exactly what you are doing.
    #[inline]
    unsafe fn raw_mut(&mut self) -> &mut Self::Raw {
        &mut *(self as *mut Self).cast()
    }

    /// Wraps a reference to the raw il2cpp type
    ///
    /// # Safety
    /// The wrapped type must be in a valid state.
    #[inline]
    unsafe fn wrap(raw: &Self::Raw) -> &Self {
        &*(raw as *const Self::Raw).cast()
    }

    /// Wraps a mutable reference to the raw il2cpp type
    ///
    /// # Safety
    /// The wrapped type must be in a valid state.
    #[inline]
    unsafe fn wrap_mut(raw: &mut Self::Raw) -> &mut Self {
        &mut *(raw as *mut Self::Raw).cast()
    }

    /// Wraps a const pointer to the raw il2cpp type
    ///
    /// # Safety
    /// The pointer must not be dangling and must stay valid for the lifetime of
    /// the returned reference if it is not null, and the wrapped type must be
    /// in a valid state.
    #[inline]
    unsafe fn wrap_ptr<'a>(ptr: *const Self::Raw) -> Option<&'a Self> {
        transmute(ptr)
    }

    /// Wraps a mut pointer to the raw il2cpp type
    ///
    /// # Safety
    /// The pointer must not be dangling and must stay valid for the lifetime of
    /// the returned mutable reference if it is not null, and the wrapped type
    /// must be in a valid state.
    #[inline]
    unsafe fn wrap_ptr_mut<'a>(ptr: *mut Self::Raw) -> Option<&'a mut Self> {
        transmute(ptr)
    }
}

/// Unboxes a value type stored as an [`Il2CppObject`]
///
/// # Safety
/// The object must be of the valid type and cointain a valid value.
///
/// `Object::Unbox` (the underlying il2cpp function) does not allocate memory,
/// so the returned value is a copy of the value stored in the object.
#[inline]
pub unsafe fn unbox<T: ValueType>(object: &Il2CppObject) -> T {
    let address = object as *const Il2CppObject as usize;
    let ptr = (address + size_of::<Il2CppObject>()) as *const T;
    ptr.read_unaligned()
}

/// Boxes a value type into a [`BoxedValue<T>`]
/// # Safety
/// The provided value must be a valid value of the given type.
///
/// `Object::Box` (the underlying il2cpp function) allocates memory for the
/// boxed object, so the returned pointer is managed by the il2cpp GC
#[inline]
pub unsafe fn value_box_alloc<T: ValueType>(this: &T) -> Gc<BoxedValue<T>> {
    // TODO: WrapRaw for T?
    let object = functions::value_box(
        T::class().raw() as *const Il2CppClass as *mut Il2CppClass,
        (this as *const T).cast::<c_void>(),
    );
    object.cast::<BoxedValue<T>>().into()
}

/// Boxes a value type into an [`Il2CppObject`] without allocating or copying,
/// mirroring beatsaber-hook's `to_object<Box = true>(fake_box = true)`
///
/// Rather than calling `Object::Box`, this pretends an [`Il2CppObject`]
/// header sits right before `this`, offsetting its address back by the
/// header's size. Unboxing the result (which adds the header size back)
/// yields `this` again, but the header itself is never written, so the
/// returned pointer must never be read as a real object (e.g. its class) -
/// only unboxed back into `T`. It is only valid for as long as `this` is.
///
/// # Safety
/// The provided value must be a valid value of the given type.
#[inline]
pub unsafe fn fake_value_box<T: ValueType>(this: &T) -> *mut Il2CppObject {
    // https://github.com/QuestPackageManager/beatsaber-hook/blob/7632eb7bf2634dabbf3cade1df140e5d93f48845/shared/types.hpp#L227-L228
    // Real boxing by necessity copies the struct into the boxed object, in addition
    // to having higher overhead, so modifications would have to be copied back
    // to the original object
    let address = this as *const T as usize;
    (address - size_of::<Il2CppObject>()) as *mut Il2CppObject
}

/// A generic parameter's own metadata entry - matches
/// `vm/GlobalMetadataFileInternals.h`'s `Il2CppGenericParameter` exactly
/// (`ownerIndex: GenericContainerIndex`, `nameIndex: StringIndex`,
/// `constraintsStart: GenericParameterConstraintIndex`,
/// `constraintsCount: int16_t`, `num: uint16_t`, `flags: uint16_t`). On
/// `il2cpp_v29`/`il2cpp_v31`, this is what `Il2CppType`'s
/// `genericParameterHandle` union member points to: il2cpp mmaps its global
/// metadata file directly into the process and hands out pointers straight
/// into that mapping, so this struct's layout mirrors the file format
/// exactly rather than some separately-materialized runtime representation -
/// see
/// [`Il2CppType::generic_parameter_index`](crate::Il2CppType::generic_parameter_index).
///
/// With the `bindgen` feature, this is instead generated directly from that
/// header (added to `wrapper.h`), which is the same layout by construction;
/// this hand-written copy only backs the vendored, pre-generated
/// `types_v31`/`types_v24`/`types_2018` fallback used when `bindgen` is off.
#[cfg(not(feature = "bindgen"))]
#[repr(C)]
pub struct Il2CppGenericParameter {
    pub owner_index: i32,
    pub name_index: i32,
    pub constraints_start: i16,
    pub constraints_count: i16,
    /// This parameter's 0-based position in its owner's parameter list -
    /// e.g. for `Foo<T, U>`, `T`'s `num` is 0 and `U`'s is 1.
    pub num: u16,
    pub flags: u16,
}

/// A generic container's own metadata entry - matches
/// `vm/GlobalMetadataFileInternals.h`'s `Il2CppGenericContainer` exactly
/// (`ownerIndex: i32`, `type_argc: i32`, `is_method: i32`,
/// `genericParameterStart: GenericParameterIndex`). On `il2cpp_v29`/
/// `il2cpp_v31`, a generic (non-inflated) [`MethodInfo`]'s
/// `genericContainerHandle` union member points directly at one of these in
/// the mmap'd global metadata file, the same way `Il2CppType`'s
/// `genericParameterHandle` points at an [`Il2CppGenericParameter`] - see
/// [`MethodInfo::generic_parameter_count`](crate::MethodInfo::generic_parameter_count).
///
/// With the `bindgen` feature, this is instead generated directly from that
/// header, which is the same layout by construction; this hand-written copy
/// only backs the vendored, pre-generated `types_v31`/`types_v24`/
/// `types_2018` fallback used when `bindgen` is off.
#[cfg(not(feature = "bindgen"))]
#[repr(C)]
pub struct Il2CppGenericContainer {
    pub owner_index: i32,
    pub type_argc: i32,
    pub is_method: i32,
    pub generic_parameter_start: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `unbox` must read from `size_of::<Il2CppObject>()` bytes past the
    /// object pointer, not from the pointer itself.
    #[test]
    fn unbox_reads_the_value_placed_after_the_header() {
        #[repr(align(16))]
        struct Buf([u8; 64]);

        let mut buf = Buf([0; 64]);
        let value: u32 = 0xCAFE_F00D;

        unsafe {
            let value_ptr = buf
                .0
                .as_mut_ptr()
                .add(size_of::<Il2CppObject>())
                .cast::<u32>();
            value_ptr.write_unaligned(value);

            let object = &*buf.0.as_ptr().cast::<Il2CppObject>();
            let unboxed: u32 = unbox(object);
            assert_eq!(unboxed, value);
        }
    }

    /// `fake_value_box` must not allocate: the pointer it returns is `this`'s
    /// own address, shifted back by exactly one header's worth of bytes.
    #[test]
    fn fake_value_box_offsets_the_address_back_by_the_header_size() {
        let value: u64 = 42;

        unsafe {
            let object = fake_value_box(&value);
            let object_addr = object as usize;
            let value_addr = &value as *const u64 as usize;
            assert_eq!(object_addr + size_of::<Il2CppObject>(), value_addr);
        }
    }

    /// Boxing a value with `fake_value_box` and reading it back with `unbox`
    /// must reproduce the original value, since the two apply opposite
    /// offsets of the same size.
    #[test]
    fn fake_value_box_round_trips_through_unbox() {
        let value: u64 = 0xDEAD_BEEF_CAFE_F00D;

        unsafe {
            let object = fake_value_box(&value);
            let unboxed: u64 = unbox(&*object);
            assert_eq!(unboxed, value);
        }
    }
}
