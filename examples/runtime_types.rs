use std::ffi::c_void;
use std::ptr::null_mut;

use quest_hook::hook;
use quest_hook::libil2cpp::{
    raw, Gc, Il2CppClass, Il2CppException, Il2CppObject, Il2CppReflectionType, MethodInfo, Type,
    WrapRaw,
};
use tracing::debug;

#[hook("UnityEngine", "Component", "get_transform")]
fn get_transform(this: &mut Il2CppObject) -> Gc<Il2CppObject> {
    // Without generics: `this`'s actual class (`Component`) is never named
    // as a Rust type - only `Il2CppObject`, the crate's own "any C# object"
    // wrapper, is - and the result is `Gc<Il2CppObject>` rather than a
    // specific mapped return type.
    let game_object: Gc<Il2CppObject> = this.invoke("get_gameObject", ()).unwrap();
    debug!("gameObject: {:?}", game_object.as_ref().map(|o| o.class()));

    // With generics: `Component.GetComponent<T>()`, where `T` is
    // `UnityEngine.Rigidbody` - looked up by name at runtime
    // (`Il2CppClass::find`) rather than a compile-time Rust type like
    // `examples/generics.rs`'s `Rigidbody` struct.
    let rigidbody_class =
        Il2CppClass::find("UnityEngine", "Rigidbody").expect("Rigidbody should exist");

    let get_component = this
        .class()
        .find_method_unchecked("GetComponent", 0)
        .expect("Component.GetComponent<T>() should exist");

    let get_component_rigidbody = make_generic_method(get_component, rigidbody_class)
        .expect("MakeGenericMethod should succeed");

    let rigidbody: Gc<Il2CppObject> =
        unsafe { get_component_rigidbody.invoke_unchecked(&mut *this, ()) }.unwrap();
    debug!("rigidbody: {:?}", rigidbody.as_ref().map(|r| r.class()));

    get_transform.original(this)
}

/// Instantiates a generic method with a single type argument found at
/// runtime, rather than a compile-time `G: Generics` -
/// [`MethodInfo::make_generic`] only needs `G` to build the `System.Type[]`
/// that `MakeGenericMethod` takes; this builds that array by hand off
/// `class` instead, the same way `Generics for T: Type`'s `type_array` does
/// off `T::class()`.
fn make_generic_method(
    method: &'static MethodInfo,
    class: &'static Il2CppClass,
) -> Option<&'static MethodInfo> {
    let types = unsafe { raw::array_new(Il2CppReflectionType::class().raw(), 1) }.unwrap();
    unsafe {
        (((types as *mut _ as isize) + (raw::kIl2CppSizeOfArray as isize))
            as *mut &Il2CppReflectionType)
            .write_unaligned(class.ty().reflection_object());
    }

    let reflection_method = method.reflection_object();
    let make_generic_method = reflection_method
        .class()
        .find_method_unchecked("MakeGenericMethod", 2)
        .unwrap();

    let ret = unsafe {
        make_generic_method.invoke_raw(
            null_mut(),
            [
                reflection_method as *const _ as *mut c_void,
                (types as *mut raw::Il2CppArray).cast(),
            ]
            .as_mut(),
        )
    };

    let obj = match ret {
        Ok(Some(obj)) => obj,
        Ok(None) => return None,
        Err(e) => panic!("MakeGenericMethod threw: {:?}", unsafe {
            Il2CppException::wrap_mut(e)
        }),
    };

    Some(unsafe {
        let refl = &*(obj as *mut raw::Il2CppObject).cast::<raw::Il2CppReflectionMethod>();
        MethodInfo::wrap(raw::method_get_from_reflection(refl))
    })
}

#[no_mangle]
pub extern "C" fn setup() {
    quest_hook::setup_log("runtime_types");
}

#[no_mangle]
pub extern "C" fn load() {
    get_transform.install().unwrap();
}
