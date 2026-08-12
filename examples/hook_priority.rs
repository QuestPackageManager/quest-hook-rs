use libil2cpp::unsafe_impl_reference_type;
use quest_hook::hook;
use quest_hook::libil2cpp::{Il2CppObject, Il2CppString, Type};
use tracing::debug;

// Mirrors what a codegen tool (e.g. bs-cordl-rust) already generates for a
// C# class: a marker implementing `libil2cpp::Type` (giving it NAMESPACE/
// CLASS_NAME, among other things), with each method as a real, callable
// inherent function - no extra wrapper type needed for a `#[hook]` to target
// one of them directly, addressed the same way C++'s MAKE_HOOK_MATCH
// addresses a method through `&ClassName::MethodName`.
struct SceneManager;

unsafe_impl_reference_type!(in quest_hook::libil2cpp for SceneManager => UnityEngine.SceneManagement.SceneManager);

impl SceneManager {
    /// The actual method - genuinely callable, with the exact signature any
    /// `#[hook]` targeting it must declare. `public static bool
    /// SetActiveScene(Scene scene)` per Unity's docs - a static method, so
    /// it's called through `Self::class()` rather than taking `this`. Never
    /// called in this example, which only demonstrates the
    /// `#[hook(SceneManager::SetActiveScene)]` targeting syntax below.
    #[allow(dead_code, non_snake_case)]
    fn SetActiveScene(scene: &mut Il2CppObject) -> bool {
        Self::class().invoke("SetActiveScene", (scene,)).unwrap()
    }
}

// A hook's own name defaults to its function name and its own namespace
// defaults to the crate name, both used only to let other hooks order
// themselves relative to it (unrelated to the "UnityEngine.SceneManagement",
// "SceneManager", "SetActiveScene" target below, which says *what* C# method
// to hook).
#[hook("UnityEngine.SceneManagement", "SceneManager", "SetActiveScene")]
fn log_scene_change(scene: &mut Il2CppObject) -> bool {
    let name: &Il2CppString = scene.invoke("get_name", ()).unwrap();
    debug!("scene changing to {}", name);

    log_scene_change.original(scene)
}

// Installs so it always runs before `log_scene_change`. `before`/`after`
// filter by name, namespace, or both: a bare name like this one matches
// `log_scene_change` in any namespace, `"my_crate::"` would match any hook
// named anything in the `my_crate` namespace, and `"my_crate::some_hook"`
// would require both. Priority is only meaningfully enforced by the
// `flamingo` backend, which supports multiple hooks per target; other
// backends install normally but ignore it.
//
// Also targets `SceneManager::SetActiveScene`: since both this hook and
// `log_scene_change` are checked against that same real method, they're
// implicitly checked against each other too, without either depending on
// the other's presence.
#[hook(
    SceneManager::SetActiveScene,
    before = "log_scene_change",
    after = "my_crate::some_other_hook",
    after = "my_crate::"
)]
fn validate_scene_change(scene: &mut Il2CppObject) -> bool {
    debug!("validating scene change");

    validate_scene_change.original(scene)
}

#[no_mangle]
pub extern "C" fn setup() {
    quest_hook::setup_log("hook priority");
}

#[no_mangle]
pub extern "C" fn load() {
    // `install` returns a `HookHandle` that can later remove just this hook,
    // independently of any other hooks sharing its target.
    let handle = validate_scene_change.install().unwrap();
    log_scene_change.install().unwrap();

    // Uninstalling is as simple as calling `uninstall` on the handle; here
    // we immediately remove `validate_scene_change` again, leaving
    // `log_scene_change` installed and running on its own. This only
    // succeeds on backends that support removing a hook (currently just
    // `flamingo`); other backends always return `Err`.
    if let Err(err) = handle.uninstall() {
        debug!("could not uninstall validate_scene_change: {err}");
    }
}
