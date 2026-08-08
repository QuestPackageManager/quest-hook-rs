#!/usr/bin/env bash
# Dumps the address and raw instruction bytes of an exported symbol from an
# il2cpp .so, formatted for pasting into xref unit tests (e.g.
# `libil2cpp/src/xref/arm64/disasm.rs`, `libil2cpp/src/xref/arm64/gc.rs`).
#
# The host `objdump` generally can't disassemble aarch64 (it errors with
# "can't use supplied machine aarch64"), so this uses `llvm-objdump`
# (`apt install llvm` / already bundled with most LLVM installs) instead.
#
# Usage:
#   dump_xref_bytes.sh <path-to-libil2cpp.so> <symbol-name> [instruction-count]
#
# Example:
#   dump_xref_bytes.sh tests/il2cpp_v31/android/libil2cpp.so il2cpp_gc_alloc_fixed 4
#
# To follow a chain of branches (e.g. gc_alloc_fixed -> its branch target ->
# *that* target's branch), re-run with the target address's containing
# symbol, or just add `--disassemble-symbols=<hex-addr>`-style manual
# `llvm-objdump` invocations - see the Ghidra notes in this directory's
# README for the manual/no-script equivalent.

set -euo pipefail

if [[ $# -lt 2 ]]; then
    echo "usage: $0 <path-to-libil2cpp.so> <symbol-name> [instruction-count]" >&2
    exit 1
fi

so_path=$1
symbol=$2
count=${3:-4}

llvm_objdump=$(command -v llvm-objdump || true)
if [[ -z "$llvm_objdump" ]]; then
    for candidate in /usr/lib/llvm-*/bin/llvm-objdump; do
        if [[ -x "$candidate" ]]; then
            llvm_objdump=$candidate
            break
        fi
    done
fi
if [[ -z "${llvm_objdump:-}" ]]; then
    echo "error: llvm-objdump not found (try: apt install llvm)" >&2
    exit 1
fi

disasm=$("$llvm_objdump" -d --triple=aarch64 --disassemble-symbols="$symbol" "$so_path" 2>&1)

if ! grep -q "<$symbol>:" <<<"$disasm"; then
    echo "error: symbol '$symbol' not found in $so_path" >&2
    echo "(if it's not dynamically exported, look it up manually - see the README)" >&2
    exit 1
fi

echo "=== $symbol ==="
echo "$disasm" | grep -A"$count" "<$symbol>:" | head -n $((count + 1))
echo

# Reassemble the disassembly into a Rust byte array + address constants,
# matching the style already used in the xref test modules.
echo "$disasm" | grep -A"$count" "<$symbol>:" | tail -n +2 | head -n "$count" | awk -v sym="$symbol" '
    NR == 1 {
        addr = $1
        sub(/:$/, "", addr)
        printf "const %s_ADDR: usize = 0x%s;\n", toupper(sym), addr
    }
    {
        word = $2
        bytes = ""
        for (i = 7; i >= 1; i -= 2) {
            bytes = bytes "0x" substr(word, i, 2) ", "
        }
        all_bytes = all_bytes bytes
    }
    END {
        n = split(all_bytes, b, ", ")
        printf "const %s_BYTES: [u8; %d] = [%s];\n", toupper(sym), (n - 1), all_bytes
    }
'
