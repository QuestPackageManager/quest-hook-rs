use quest_hook::hook;
use quest_hook::libil2cpp::{Gc, Il2CppClass, Il2CppObject};
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
    // `examples/generics.rs`'s `Rigidbody` struct. `MethodInfo::make_generic_with`
    // takes the generic arguments as classes found this way instead of a
    // compile-time `G: Generics`.
    let rigidbody_class =
        Il2CppClass::find("UnityEngine", "Rigidbody").expect("Rigidbody should exist");

    let get_component = this
        .class()
        .find_method_unchecked("GetComponent", 0)
        .expect("Component.GetComponent<T>() should exist");

    let get_component_rigidbody = get_component
        .make_generic_with(&[rigidbody_class])
        .expect("MakeGenericMethod should succeed")
        .expect("GetComponent<T>() is generic, so this always succeeds");

    let rigidbody: Gc<Il2CppObject> =
        unsafe { get_component_rigidbody.invoke_unchecked(&mut *this, ()) }.unwrap();
    debug!("rigidbody: {:?}", rigidbody.as_ref().map(|r| r.class()));

    get_transform.original(this)
}

#[no_mangle]
pub extern "C" fn setup() {
    quest_hook::setup_log("runtime_types");
}

#[no_mangle]
pub extern "C" fn load() {
    get_transform.install().unwrap();
}
