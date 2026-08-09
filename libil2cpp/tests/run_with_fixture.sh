#!/usr/bin/env bash
# Cargo `runner` wrapper (see /.cargo/config.toml) for tests that dlopen the
# `tests/il2cpp_v31/<fixture>/` binaries by bare name (e.g. `GameAssembly.so`).
#
# `LD_LIBRARY_PATH` (and platform equivalents) must be set *before* the test
# process starts for `dlopen`/`LoadLibrary` to see it - glibc parses it once
# at process startup, so setting it from within the already-running test has
# no effect on later `dlopen` calls. Hence this wrapper, rather than an
# `env::set_var` call inside the test itself.
#
# Usage (from .cargo/config.toml): run_with_fixture.sh <fixture-dir> <test-binary> [args...]
set -euo pipefail

fixture_dir_name="$1"
shift

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fixture_dir="$script_dir/il2cpp_v31/$fixture_dir_name"

case "$(uname -s)" in
    Darwin*)
        export DYLD_LIBRARY_PATH="$fixture_dir${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
        ;;
    *)
        export LD_LIBRARY_PATH="$fixture_dir${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
        ;;
esac

exec "$@"
