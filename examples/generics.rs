use quest_hook::hook;
use quest_hook::libil2cpp::{unsafe_impl_reference_type, Gc, Il2CppClass, Il2CppObject, Type};
use tracing::debug;

// A C# class (UnityEngine.Component) - `GetComponent<T>()`, the generic
// *method* instantiated below, is declared here.
#[repr(C)]
pub struct Component {
    object: Il2CppObject,
}
unsafe_impl_reference_type!(in quest_hook::libil2cpp for Component.object => UnityEngine.Component);

// A C# class (UnityEngine.Rigidbody) - the concrete type argument used for
// both the generic class and generic method below. Nothing special is
// needed to use a mapped type as a generic argument.
#[repr(C)]
pub struct Rigidbody {
    object: Il2CppObject,
}
unsafe_impl_reference_type!(in quest_hook::libil2cpp for Rigidbody.object => UnityEngine.Rigidbody);

#[hook("UnityEngine", "Component", "get_transform")]
fn get_transform(this: &mut Component) -> Gc<Il2CppObject> {
    // Generic *class*: `System.Collections.Generic.List<Rigidbody>`,
    // resolved purely at runtime via reflection (`Type.MakeGenericType`) -
    // unlike `custom_type.rs`'s `List<T>`, no hand-written Rust type is
    // needed for this one, just the open generic's name plus a type
    // argument.
    let list_class = Il2CppClass::find_generic::<Rigidbody>("System.Collections.Generic", "List")
        .expect("List<Rigidbody> should exist");
    debug!("List<Rigidbody> is {list_class}");

    // Generic *method*: `Component.GetComponent<T>()`. `T` is a type
    // parameter, not a regular parameter, so the *un-instantiated*
    // definition still has 0 parameters - found by name and parameter count
    // rather than `find_method`, whose type checking assumes an
    // already-concrete signature.
    let get_component = Component::class()
        .find_method_unchecked("GetComponent", 0)
        .expect("Component.GetComponent<T>() should exist");

    // Instantiate it for `T = Rigidbody`, the same way C#'s
    // `MethodInfo.MakeGenericMethod` would.
    let get_component_rigidbody = get_component
        .make_generic::<Rigidbody>()
        .expect("MakeGenericMethod should succeed")
        .expect("GetComponent<T>() is generic, so this always succeeds");

    // Call the now-concrete method - through `invoke_unchecked` rather than
    // the typed `invoke`, since (like `find_method_unchecked` above) type
    // checking assumes an already-concrete signature.
    let rigidbody: Gc<Rigidbody> =
        unsafe { get_component_rigidbody.invoke_unchecked(&mut *this, ()) }.unwrap();
    debug!(
        "found rigidbody: {:?}",
        rigidbody.as_ref().map(|r| r.class())
    );

    get_transform.original(this)
}

#[no_mangle]
pub extern "C" fn setup() {
    quest_hook::setup_log("generics");
}

#[no_mangle]
pub extern "C" fn load() {
    get_transform.install().unwrap();
}
