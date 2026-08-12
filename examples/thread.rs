use std::time::Duration;

use quest_hook::libil2cpp::thread::spawn_attached;
use quest_hook::libil2cpp::{Gc, Il2CppObject, ObjectExt};
use tracing::debug;

#[no_mangle]
pub extern "C" fn setup() {
    quest_hook::setup_log("thread");
}

#[no_mangle]
pub extern "C" fn load() {
    // `load` itself already runs on a thread the mod loader attached to
    // il2cpp before calling us - this is only needed because we're about to
    // hand work off to a *new* OS thread, which starts out not attached.
    let handle = spawn_attached(|| {
        // Pretend this is slow, off-game-thread work (disk IO, a network
        // request, ...) before we need a fresh C# object.
        std::thread::sleep(Duration::from_millis(100));

        // `spawn_attached` already attached this thread, so il2cpp's GC can
        // scan its stack. That matters here specifically because `obj` is a
        // live reference to a GC-managed object: without attaching, a
        // collection running concurrently on another thread wouldn't know
        // this local is keeping it alive, and could free it right under us.
        let obj: Gc<Il2CppObject> = Il2CppObject::new(());
        debug!("(background thread) allocated a {:?}", obj.class());
        obj
    });

    // `spawn_attached` hands back a plain `JoinHandle` - joining it here just
    // keeps this example self-contained. Real usage would usually let it run
    // and finish on its own rather than blocking `load` on it. `obj` is now
    // held on *this* thread instead, which needs to stay attached itself for
    // as long as it keeps the reference alive - true here since `load` was
    // already called on an attached thread.
    let obj = handle.join().expect("background thread panicked");
    debug!("received object from background thread: {:?}", obj.class());
}
