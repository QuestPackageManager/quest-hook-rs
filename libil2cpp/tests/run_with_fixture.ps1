# Cargo `runner` wrapper (see /.cargo/config.toml) for tests that dlopen the
# `tests/il2cpp_v31/<fixture>/` binaries by bare name (e.g. `GameAssembly.dll`).
#
# `PATH` must be set *before* the test process starts for `LoadLibrary` to
# see it - setting it from within the already-running test has no effect on
# later `LoadLibrary` calls made by that same process. Hence this wrapper,
# rather than an `env::set_var` call inside the test itself.
#
# Usage (from .cargo/config.toml): run_with_fixture.ps1 <fixture-dir> <test-binary> [args...]
param(
    [Parameter(Mandatory = $true)][string]$FixtureDirName,
    [Parameter(Mandatory = $true, ValueFromRemainingArguments = $true)][string[]]$TestArgs
)

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$FixtureDir = Join-Path (Join-Path $ScriptDir "il2cpp_v31") $FixtureDirName

$env:PATH = "$FixtureDir;$env:PATH"

& $TestArgs[0] $TestArgs[1..($TestArgs.Length - 1)]
exit $LASTEXITCODE
