use libil2cpp::{Gc, Il2CppArray, ObjectExt, RefType, Type};
use quest_hook::hook;
use quest_hook::libil2cpp::{unsafe_impl_reference_type, unsafe_impl_value_type, Il2CppObject};
use tracing::debug;

// A C# struct (UnityEngine.Vector3) - a plain, non-generic value type.
#[derive(Debug)]
#[repr(C)]
pub struct Vector3 {
    x: f32,
    y: f32,
    z: f32,
}
unsafe_impl_value_type!(in quest_hook::libil2cpp for Vector3 => UnityEngine.Vector3);

// A C# class (UnityEngine.Transform) - a plain, non-generic reference type.
// Naming its `object` field lets `unsafe_impl_reference_type!` implement
// `AsRef`/`AsMut` to `Il2CppObject` for us, so `RefType::as_object_mut()`
// reaches `invoke` on a `&mut Transform` below.
#[repr(C)]
pub struct Transform {
    object: Il2CppObject,
}
unsafe_impl_reference_type!(in quest_hook::libil2cpp for Transform.object => UnityEngine.Transform);

// A C# struct (System.Nullable<T>) - a generic value type. Its layout
// mirrors the real Nullable<T>: a `bool` flag followed by the value itself.
#[derive(Debug)]
#[repr(C)]
pub struct Nullable<T: Type> {
    has_value: bool,
    value: T,
}
unsafe_impl_value_type!(in quest_hook::libil2cpp for Nullable<T> => System.Nullable<T>);

// A C# class (System.Collections.Generic.List<T>) - a generic reference
// type. Its layout mirrors the real List<T> object: an Il2CppObject header
// followed by its fields, and the Rust generic `T` maps onto C#'s generic
// parameter.
#[repr(C)]
pub struct List<T: Type> {
    object: Il2CppObject,
    items: *mut Il2CppArray<T>,
    size: i32,
}
unsafe_impl_reference_type!(in quest_hook::libil2cpp for List<T>.object => System.Collections.Generic.List<T>);

#[hook("UnityEngine", "RigidBody", "set_position")]
fn set_position(this: &mut Il2CppObject, new_position: Vector3) {
    let old_position: Vector3 = this.invoke("get_position", ()).unwrap();
    debug!("{:?} -> {:?}", old_position, new_position);

    // `Transform` (plain reference type): `RefType::as_object_mut()` reaches
    // `Il2CppObject::invoke` through the `AsMut<Il2CppObject>` the macro
    // generated.
    let transform: &mut Transform = this.invoke("get_transform", ()).unwrap();
    // `Nullable<Vector3>` (generic value type): a target the rigidbody is
    // easing towards, if any.
    let target_position: Nullable<Vector3> = transform
        .as_object_mut()
        .invoke("get_targetPosition", ())
        .unwrap();
    debug!("target position: {:?}", target_position);

    // `List<Vector3>` (generic reference type): `ObjectExt::new` is the
    // equivalent of C#'s `new List<Vector3>()`.
    let recent_positions: Gc<List<Vector3>> = List::new(());
    debug!(
        "tracking recent positions in {:?}",
        recent_positions.as_object().class()
    );

    set_position.original(this, new_position)
}

#[no_mangle]
pub extern "C" fn setup() {
    quest_hook::setup_log("custom type");
}

#[no_mangle]
pub extern "C" fn load() {
    set_position.install().unwrap();
}
