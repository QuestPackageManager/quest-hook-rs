#!/bin/pwsh

# $ENV:BINDGEN_EXTRA_CLANG_ARGS="--target=aarch64-linux-android21 -I./extern/includes/libil2cpp/il2cpp/libil2cpp -I$ENV:ANDROID_NDK_HOME/sysroot/usr/include -I$ENV:ANDROID_NDK_HOME/sysroot/usr/include/aarch64-linux-android"
# bindgen wrapper.h -o bindings.rs --wrap-unsafe-ops --sort-semantically
$ENV:RUN_BINDGEN=1 & cargo ndk --bindgen -t arm64-v8a build