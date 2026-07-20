fn main() {
    if std::env::var("CARGO_FEATURE_INLINE_HOOK").is_err() {
        // The vendored And64InlineHook/inlineHook.c backend is only needed
        // when the `inline_hook` feature is selected.
        return;
    }

    let target = std::env::var("TARGET").unwrap();
    if target == "aarch64-linux-android" {
        cc::Build::new()
            .file("beatsaber-hook/shared/inline-hook/And64InlineHook.cpp")
            .compile("inline_hook");
    } else if target == "armv7-linux-androideabi" {
        cc::Build::new()
            .file("beatsaber-hook/shared/inline-hook/inlineHook.c")
            .include("beatsaber-hook/shared/inline-hook")
            .compile("inline_hook");
    }
}
