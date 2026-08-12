use quest_hook::hook;
use quest_hook::libil2cpp::{Il2CppObject, Il2CppString, Il2CppType, Type};

struct MyClass;

unsafe impl Type for MyClass {
    type Held<'a> = Option<&'a mut Self>;
    type HeldRaw = *mut Self;

    const NAMESPACE: &'static str = "MyNamespace";
    const CLASS_NAME: &'static str = "MyClass";

    fn matches_reference_argument(ty: &Il2CppType) -> bool {
        ty.class().is_assignable_from(Self::class())
    }
    fn matches_value_argument(_: &Il2CppType) -> bool {
        false
    }
    fn matches_reference_parameter(ty: &Il2CppType) -> bool {
        Self::class().is_assignable_from(ty.class())
    }
    fn matches_value_parameter(_: &Il2CppType) -> bool {
        false
    }
}

impl MyClass {
    #[allow(non_snake_case)]
    fn MyMethod(this: &mut Il2CppObject, value: &mut Il2CppObject) -> bool {
        let _ = (this, value);
        unimplemented!()
    }
}

// `MyMethod`'s second parameter is `&mut Il2CppObject`, not `&mut
// Il2CppString` - this hook declares the wrong type for it, so it should
// fail to compile.
#[hook(MyClass::MyMethod)]
fn my_hook(this: &mut Il2CppObject, value: &mut Il2CppString) -> bool {
    my_hook.original(this, value)
}

fn main() {}
