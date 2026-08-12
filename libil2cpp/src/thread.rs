//! Helpers for running Rust code on native threads that talk to il2cpp.
//!
//! Attaching a thread registers it with il2cpp's GC so the GC can scan its
//! stack for references when deciding what's still reachable. A thread that
//! isn't attached is invisible to that scan: a GC-managed object referenced
//! only from a local on such a thread (a [`Gc<T>`](crate::Gc) held in a
//! local, say) can be collected out from under it while a collection runs
//! concurrently. Threads Unity itself spawns (the main thread, Unity
//! job-system workers, ...) are already attached; a thread *you* spawn
//! (`std::thread::spawn`, a native callback arriving on some other
//! library's thread, ...) is not, and should attach first if it's going to
//! hold GC references across a potential collection. Ports beatsaber-hook's
//! `threading.hpp`.
//!
//! Unlike upstream's `attach_thread`, this only attaches to **il2cpp** -
//! it doesn't also attach the thread to a JVM (`JNIEnv::AttachCurrentThread`
//! on Android), since this crate has no JNI/JVM handle to do that with.
//! Calling into JNI from a thread attached here still needs its own
//! `AttachCurrentThread` call.
//!
//! See [`crate::r#async`] for an `.await`-able equivalent of
//! [`spawn_attached`], built on top of [`attached_invoke`].

use std::thread::{self, JoinHandle};

use crate::raw;

/// RAII guard produced by [`attach`] - detaches the thread it was created
/// for when dropped (including when unwinding from a panic).
pub struct AttachedThread {
    thread: &'static mut raw::Il2CppThread,
}

impl Drop for AttachedThread {
    fn drop(&mut self) {
        unsafe { raw::thread_detach(self.thread) };
    }
}

/// Attaches the current thread to the il2cpp runtime, returning a guard
/// that detaches it again on drop.
///
/// <https://github.com/QuestPackageManager/beatsaber-hook/blob/7632eb7bf2634dabbf3cade1df140e5d93f48845/shared/threading.hpp#L36-L45>
///
/// # Safety
/// The calling thread must not already be attached - attaching an
/// already-attached thread, or detaching one still in use elsewhere (by
/// dropping a second, overlapping guard for it), is not a scenario il2cpp
/// is documented to handle gracefully.
pub unsafe fn attach() -> AttachedThread {
    let domain = unsafe { raw::domain_get() };
    let thread = unsafe { raw::thread_attach(domain) }.expect("il2cpp_thread_attach returned null");
    AttachedThread { thread }
}

/// Runs `f` with the current thread attached to il2cpp for the duration,
/// detaching again once `f` returns (or panics) - see [`attach`].
///
/// <https://github.com/QuestPackageManager/beatsaber-hook/blob/7632eb7bf2634dabbf3cade1df140e5d93f48845/shared/threading.hpp#L57-L65>
///
/// # Safety
/// Same as [`attach`]: the calling thread must not already be attached.
pub unsafe fn attached_invoke<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = unsafe { attach() };
    f()
}

/// Spawns a new OS thread that attaches itself to il2cpp before running
/// `f` and detaches again before exiting - the safe, ergonomic entry point
/// built on [`attached_invoke`] (safe because a freshly spawned OS thread
/// is never already attached, satisfying its safety requirement by
/// construction).
///
/// See [`crate::r#async::il2cpp_async`] for a version that returns an
/// `.await`-able [`Future`](std::future::Future) instead of a
/// [`JoinHandle`].
///
/// <https://github.com/QuestPackageManager/beatsaber-hook/blob/7632eb7bf2634dabbf3cade1df140e5d93f48845/shared/threading.hpp#L81-L101>
pub fn spawn_attached<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    thread::spawn(move || unsafe { attached_invoke(f) })
}
