use quest_hook::hook;
use quest_hook::libil2cpp::{Il2CppObject, Il2CppString};
use tracing::debug;

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
#[hook(
    "UnityEngine.SceneManagement",
    "SceneManager",
    "SetActiveScene",
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
    quest_hook::setup("hook priority");
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
