//! Locates a loaded module's mapped memory image, by file name, in this
//! process. Used to expose the live, relocated bytes of the loaded
//! `libil2cpp`/`GameAssembly` binary for xref pattern scanning - as opposed
//! to its on-disk file, which isn't what's actually executed at runtime.
//!
//! Each platform backend below exposes the same `find(name) -> Option<&'static
//! [u8]>` signature, so [`super::get_il2cpp_bytes`] doesn't need to know which
//! one it's calling.
//!
//! Entirely vibe coded by Claude

/// ELF (Linux/Android) backend - walks the loaded program headers via
/// `dl_iterate_phdr`, which every ELF module's dynamic linker already
/// maintains, rather than parsing `/proc/self/maps` text.
#[cfg(any(target_os = "linux", target_os = "android"))]
mod imp {
    use std::ffi::{c_char, c_int, c_void, CStr};

    struct Search<'a> {
        name: &'a str,
        range: Option<(usize, usize)>,
    }

    unsafe extern "C" fn callback(
        info: *mut libc::dl_phdr_info,
        _size: usize,
        data: *mut c_void,
    ) -> c_int {
        let search = unsafe { &mut *data.cast::<Search<'_>>() };
        let info = unsafe { &*info };

        let module_name: &str = if info.dlpi_name.is_null() {
            ""
        } else {
            unsafe { CStr::from_ptr(info.dlpi_name as *const c_char) }
                .to_str()
                .unwrap_or("")
        };

        if module_name.is_empty() || !module_name.ends_with(search.name) {
            return 0;
        }

        let phdrs = unsafe { std::slice::from_raw_parts(info.dlpi_phdr, info.dlpi_phnum as usize) };
        let mut range: Option<(usize, usize)> = None;
        for phdr in phdrs {
            if phdr.p_type != libc::PT_LOAD {
                continue;
            }
            let start = info.dlpi_addr as usize + phdr.p_vaddr as usize;
            let end = start + phdr.p_memsz as usize;
            range = Some(match range {
                Some((lo, hi)) => (lo.min(start), hi.max(end)),
                None => (start, end),
            });
        }

        search.range = range;
        // A module only has one `dl_phdr_info` entry, so once we've matched
        // it by name there's nothing left to search - stop iterating.
        1
    }

    /// # Safety
    /// The returned slice spans every byte between the module's lowest and
    /// highest `PT_LOAD` addresses, including any gaps between segments with
    /// different permissions - reading it is only sound if that whole range
    /// is actually mapped and readable, which holds for how
    /// `libil2cpp`/`GameAssembly` are loaded on Linux/Android.
    pub(crate) unsafe fn find(name: &str) -> Option<&'static [u8]> {
        let mut search = Search { name, range: None };
        unsafe {
            libc::dl_iterate_phdr(
                Some(callback),
                std::ptr::from_mut(&mut search).cast::<c_void>(),
            );
        }

        let (start, end) = search.range?;
        Some(unsafe { std::slice::from_raw_parts(start as *const u8, end - start) })
    }
}

/// Windows backend - looks the module up with `GetModuleHandleW`, then reads
/// `SizeOfImage` out of its own (already mapped) PE headers.
#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(lp_module_name: *const u16) -> *mut std::ffi::c_void;
    }

    /// # Safety
    /// The returned slice spans the module's entire `SizeOfImage` - the
    /// Windows loader reserves this whole range as one region up front, so
    /// it's always mapped (though not every page within it is necessarily
    /// committed/readable in unusual cases).
    pub(crate) unsafe fn find(name: &str) -> Option<&'static [u8]> {
        let wide_name: Vec<u16> = OsStr::new(name).encode_wide().chain(Some(0)).collect();
        let base = unsafe { GetModuleHandleW(wide_name.as_ptr()) };
        if base.is_null() {
            return None;
        }
        let base = base as usize;

        // IMAGE_DOS_HEADER::e_lfanew (offset to IMAGE_NT_HEADERS) is a u32
        // at offset 0x3C.
        let e_lfanew =
            unsafe { (base as *const u8).add(0x3C).cast::<u32>().read_unaligned() } as usize;
        // IMAGE_NT_HEADERS is a 4-byte Signature followed by the 20-byte
        // IMAGE_FILE_HEADER, then IMAGE_OPTIONAL_HEADER(32|64) -
        // SizeOfImage sits at the same offset (56) in both variants.
        let size_of_image = unsafe {
            (base as *const u8)
                .add(e_lfanew + 4 + 20 + 56)
                .cast::<u32>()
                .read_unaligned()
        } as usize;

        Some(unsafe { std::slice::from_raw_parts(base as *const u8, size_of_image) })
    }
}

/// macOS backend - looks the image up via the `dyld` image list, then walks
/// its `LC_SEGMENT_64` load commands to find its mapped range.
#[cfg(target_os = "macos")]
mod imp {
    use std::ffi::{c_char, c_void, CStr};

    unsafe extern "C" {
        fn _dyld_image_count() -> u32;
        fn _dyld_get_image_name(image_index: u32) -> *const c_char;
        fn _dyld_get_image_header(image_index: u32) -> *const c_void;
        fn _dyld_get_image_vmaddr_slide(image_index: u32) -> isize;
    }

    #[repr(C)]
    struct MachHeader64 {
        magic: u32,
        cputype: i32,
        cpusubtype: i32,
        filetype: u32,
        ncmds: u32,
        sizeofcmds: u32,
        flags: u32,
        reserved: u32,
    }

    #[repr(C)]
    struct LoadCommand {
        cmd: u32,
        cmdsize: u32,
    }

    #[repr(C)]
    struct SegmentCommand64 {
        cmd: u32,
        cmdsize: u32,
        segname: [u8; 16],
        vmaddr: u64,
        vmsize: u64,
        fileoff: u64,
        filesize: u64,
        maxprot: i32,
        initprot: i32,
        nsects: u32,
        flags: u32,
    }

    const LC_SEGMENT_64: u32 = 0x19;

    /// # Safety
    /// `header` must point at a valid, currently mapped Mach-O 64 header
    /// (guaranteed by `_dyld_get_image_header` for a live `dyld` image
    /// index), and `slide` must be that same image's ASLR slide.
    unsafe fn macho_image_range(header: *const c_void, slide: isize) -> Option<&'static [u8]> {
        let mach_header = unsafe { &*header.cast::<MachHeader64>() };
        let mut cursor = (header as usize) + std::mem::size_of::<MachHeader64>();

        let mut range: Option<(u64, u64)> = None;
        for _ in 0..mach_header.ncmds {
            let lc = unsafe { &*(cursor as *const LoadCommand) };
            if lc.cmd == LC_SEGMENT_64 {
                let seg = unsafe { &*(cursor as *const SegmentCommand64) };
                if seg.vmsize > 0 {
                    range = Some(match range {
                        Some((lo, hi)) => (lo.min(seg.vmaddr), hi.max(seg.vmaddr + seg.vmsize)),
                        None => (seg.vmaddr, seg.vmaddr + seg.vmsize),
                    });
                }
            }
            cursor += lc.cmdsize as usize;
        }

        let (start, end) = range?;
        let base = (start as isize + slide) as usize;
        let size = (end - start) as usize;
        Some(unsafe { std::slice::from_raw_parts(base as *const u8, size) })
    }

    /// # Safety
    /// See [`macho_image_range`] - the same "whole range must be mapped"
    /// caveat applies to the slice it returns.
    pub(crate) unsafe fn find(name: &str) -> Option<&'static [u8]> {
        let count = unsafe { _dyld_image_count() };
        for i in 0..count {
            let raw_name = unsafe { _dyld_get_image_name(i) };
            if raw_name.is_null() {
                continue;
            }
            let image_name = unsafe { CStr::from_ptr(raw_name) }.to_string_lossy();
            if !image_name.ends_with(name) {
                continue;
            }

            let header = unsafe { _dyld_get_image_header(i) };
            if header.is_null() {
                return None;
            }
            let slide = unsafe { _dyld_get_image_vmaddr_slide(i) };

            return unsafe { macho_image_range(header, slide) };
        }

        None
    }
}

pub(super) use imp::find;

#[cfg(test)]
mod tests {
    use super::find;

    #[cfg(target_os = "linux")]
    const FIXTURE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/il2cpp_v31/linux-x64/GameAssembly.so"
    );
    #[cfg(target_os = "linux")]
    const MODULE_NAME: &str = "GameAssembly.so";

    #[cfg(target_os = "windows")]
    const FIXTURE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/il2cpp_v31/windows-x64/GameAssembly.dll"
    );
    #[cfg(target_os = "windows")]
    const MODULE_NAME: &str = "GameAssembly.dll";

    /// Loads the real fixture binary checked in under
    /// `tests/il2cpp_v31/<target>/` via `libloading`, then confirms `find`
    /// can locate it in this process's own memory afterward.
    ///
    /// The `Library` is bound to a variable (not `_`) and kept alive for the
    /// whole test - `find` looks the module up by scanning this process's
    /// own loaded modules, so it needs to still be mapped when called; once
    /// the `Library` drops, the module may be unmapped.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn finds_the_freshly_loaded_fixture() {
        let _library = unsafe { libloading::Library::new(FIXTURE) }
            .unwrap_or_else(|e| panic!("failed to load fixture {FIXTURE}: {e}"));

        let bytes =
            unsafe { find(MODULE_NAME) }.expect("find() should locate the freshly-loaded fixture");
        assert!(
            !bytes.is_empty(),
            "found module should have a non-empty mapped range"
        );
    }
}
