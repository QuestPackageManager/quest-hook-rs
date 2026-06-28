#!/bin/pwsh


bindgen wrapper.h -o bindings.rs --wrap-unsafe-ops --sort-semantically -- -I./extern/includes/libil2cpp/il2cpp/libil2cpp