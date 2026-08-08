//! Small AArch64 disassembly helpers built on top of `bad64`, used to follow
//! branch instructions the way beatsaber-hook's `cs::find_nth_b` does.

use bad64::{disasm, Imm, Op, Operand};

/// Cap on how many instructions we'll scan forward from `gc_free_fixed`
/// looking for a branch to the real `GC_free`, mirroring beatsaber-hook's
/// `cs::find_nth_b` default search depth.
pub const MAX_BRANCH_SEARCH_INSTRUCTIONS: usize = 100;

fn find_nth_matching_branch_target(
    code: &[u8],
    base: usize,
    n: usize,
    max_instructions: usize,
    is_match: impl Fn(Op) -> bool,
) -> Option<usize> {
    let mut seen = 0;

    for instruction in disasm(code, base as u64).take(max_instructions) {
        let instruction = instruction.ok()?;

        if !is_match(instruction.op()) {
            continue;
        }

        seen += 1;
        if seen < n {
            continue;
        }

        return match instruction.operands().first()? {
            Operand::Label(Imm::Unsigned(addr)) => Some(*addr as usize),
            Operand::Label(Imm::Signed(addr)) => Some(*addr as usize),
            _ => None,
        };
    }

    None
}

/// Disassembles `code` (treating `base` as the address of `code[0]`) and
/// returns the resolved target of the `n`th (1-indexed) unconditional branch
/// (`B`/`BL`) instruction. Mirrors beatsaber-hook's `cs::find_nth_b`.
///
/// Gives up and returns `None` after `max_instructions` instructions, on a
/// decode error, or if fewer than `n` branches are found first.
pub fn find_nth_branch_target(
    code: &[u8],
    base: usize,
    n: usize,
    max_instructions: usize,
) -> Option<usize> {
    find_nth_matching_branch_target(code, base, n, max_instructions, |op| {
        matches!(op, Op::B | Op::BL)
    })
}

/// Like [`find_nth_branch_target`], but only counts `BL` (branch-with-link,
/// i.e. call) instructions, ignoring plain `B`. Mirrors beatsaber-hook's
/// `cs::find_nth_bl`.
pub fn find_nth_bl_target(
    code: &[u8],
    base: usize,
    n: usize,
    max_instructions: usize,
) -> Option<usize> {
    find_nth_matching_branch_target(code, base, n, max_instructions, |op| op == Op::BL)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real bytes and addresses below were extracted from
    // `libil2cpp/tests/il2cpp_v31/android/libil2cpp.so` and cross-checked
    // with `llvm-objdump -d` (the host `objdump` can't disassemble aarch64),
    // so these exercise the actual branch-encoding math against a real
    // il2cpp binary rather than hand-rolled instruction bytes. To
    // regenerate/verify, from `libil2cpp/`:
    //   tests/dump_xref_bytes.sh tests/il2cpp_v31/android/libil2cpp.so
    // il2cpp_gc_alloc_fixed 2 (needs `llvm-objdump` on PATH). See the `XREF for
    // ...` comments in `xref/arm64/gc.rs` for the resolution paths these bytes
    // exercise.

    // `il2cpp_gc_alloc_fixed` (vaddr 0x9a9794): `mov x1, xzr` then
    // `b 0x9a3884` - the first instruction isn't a branch, so this also
    // covers skipping non-matching instructions before the first match.
    const GC_ALLOC_FIXED_BYTES: [u8; 8] = [0xe1, 0x03, 0x1f, 0xaa, 0x3b, 0xe8, 0xff, 0x17];
    const GC_ALLOC_FIXED_ADDR: usize = 0x9a9794;
    const GC_ALLOC_FIXED_BRANCH_TARGET: usize = 0x9a3884;

    // `il2cpp_gc_free_fixed` (vaddr 0x9a979c): a single `b 0x9a3888`.
    const GC_FREE_FIXED_BYTES: [u8; 4] = [0x3b, 0xe8, 0xff, 0x17];
    const GC_FREE_FIXED_ADDR: usize = 0x9a979c;
    const GC_FREE_FIXED_BRANCH_TARGET: usize = 0x9a3888;

    // `Domain::GetCurrent` (vaddr 0x9e60e0): str/stp/adrp/ldr/cbnz/mov/mov,
    // then `bl 0x9a3884` - exercises skipping several non-BL instructions,
    // including a conditional branch (`cbnz`) that must not be mistaken for
    // a plain `B`/`BL`.
    const DOMAIN_GET_CURRENT_PROLOGUE_BYTES: [u8; 32] = [
        0xfe, 0x0f, 0x1e, 0xf8, 0xf4, 0x4f, 0x01, 0xa9, 0xb4, 0x63, 0x00, 0x90, 0x80, 0xe2, 0x42,
        0xf9, 0xc0, 0x01, 0x00, 0xb5, 0x00, 0x07, 0x80, 0x52, 0xe1, 0x03, 0x1f, 0xaa, 0xe2, 0xf5,
        0xfe, 0x97,
    ];
    const DOMAIN_GET_CURRENT_ADDR: usize = 0x9e60e0;
    const DOMAIN_GET_CURRENT_BL_TARGET: usize = 0x9a3884;

    #[test]
    fn find_nth_branch_target_skips_a_leading_non_branch_instruction() {
        let target = find_nth_branch_target(&GC_ALLOC_FIXED_BYTES, GC_ALLOC_FIXED_ADDR, 1, 10);
        assert_eq!(target, Some(GC_ALLOC_FIXED_BRANCH_TARGET));
    }

    #[test]
    fn find_nth_branch_target_matches_an_immediate_branch() {
        let target = find_nth_branch_target(&GC_FREE_FIXED_BYTES, GC_FREE_FIXED_ADDR, 1, 10);
        assert_eq!(target, Some(GC_FREE_FIXED_BRANCH_TARGET));
    }

    #[test]
    fn find_nth_branch_target_is_relocation_independent() {
        // The branch is PC-relative, so disassembling the same bytes at a
        // different base must shift the resolved target by the same amount.
        let base = 0x1000;
        let target = find_nth_branch_target(&GC_FREE_FIXED_BYTES, base, 1, 10).unwrap();
        let expected_delta = GC_FREE_FIXED_BRANCH_TARGET as i64 - GC_FREE_FIXED_ADDR as i64;
        assert_eq!(target as i64 - base as i64, expected_delta);
    }

    #[test]
    fn find_nth_branch_target_returns_none_past_the_last_branch() {
        // Only one branch exists in this buffer - asking for the 2nd must fail.
        let target = find_nth_branch_target(&GC_FREE_FIXED_BYTES, GC_FREE_FIXED_ADDR, 2, 10);
        assert_eq!(target, None);
    }

    #[test]
    fn find_nth_bl_target_skips_non_bl_instructions_including_conditional_branches() {
        let target = find_nth_bl_target(
            &DOMAIN_GET_CURRENT_PROLOGUE_BYTES,
            DOMAIN_GET_CURRENT_ADDR,
            1,
            10,
        );
        assert_eq!(target, Some(DOMAIN_GET_CURRENT_BL_TARGET));
    }

    #[test]
    fn find_nth_bl_target_ignores_a_plain_b() {
        // GC_ALLOC_FIXED_BYTES's only branch is an unconditional `b`, not a
        // `bl`, so find_nth_bl_target must not match it.
        let target = find_nth_branch_target(&GC_ALLOC_FIXED_BYTES, GC_ALLOC_FIXED_ADDR, 1, 10);
        assert_eq!(target, Some(GC_ALLOC_FIXED_BRANCH_TARGET));

        let bl_target = find_nth_bl_target(&GC_ALLOC_FIXED_BYTES, GC_ALLOC_FIXED_ADDR, 1, 10);
        assert_eq!(bl_target, None);
    }
}
