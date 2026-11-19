//! How much memory this computer has free, so that a very big save is opened as a tree only
//! when the tree fits. The tree of a save costs many times the file's size.

/// Memory the machine can give out now, in bytes, or `None` where it can't be asked.
pub fn available() -> Option<u64> {
    imp::available()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
mod imp {
    pub fn available() -> Option<u64> {
        let s = std::fs::read_to_string("/proc/meminfo").ok()?;
        let kb = s.lines().find_map(|l| l.strip_prefix("MemAvailable:"))?.trim().strip_suffix("kB")?.trim().parse::<u64>().ok()?;
        Some(kb * 1024)
    }
}

#[cfg(windows)]
mod imp {
    #[repr(C)]
    struct MemoryStatus {
        length: u32,
        load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page: u64,
        avail_page: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended: u64,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalMemoryStatusEx(s: *mut MemoryStatus) -> i32;
    }
    pub fn available() -> Option<u64> {
        let mut s = MemoryStatus { length: std::mem::size_of::<MemoryStatus>() as u32, load: 0, total_phys: 0, avail_phys: 0, total_page: 0, avail_page: 0, total_virtual: 0, avail_virtual: 0, avail_extended: 0 };
        // Physical memory still free, and no more than the address space a 32-bit build has.
        (unsafe { GlobalMemoryStatusEx(&mut s) } != 0).then(|| s.avail_phys.min(s.avail_virtual))
    }
}

#[cfg(target_os = "macos")]
mod imp {
    extern "C" {
        fn sysctlbyname(name: *const std::ffi::c_char, old: *mut std::ffi::c_void, oldlen: *mut usize, new: *mut std::ffi::c_void, newlen: usize) -> i32;
    }
    /// macOS has no cheap "available" figure: half of the installed memory is assumed free.
    pub fn available() -> Option<u64> {
        let mut total = 0u64;
        let mut len = std::mem::size_of::<u64>();
        let ok = unsafe { sysctlbyname(c"hw.memsize".as_ptr(), (&mut total as *mut u64).cast(), &mut len, std::ptr::null_mut(), 0) } == 0;
        ok.then_some(total / 2)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android", windows, target_os = "macos")))]
mod imp {
    pub fn available() -> Option<u64> {
        None
    }
}

/// What a tree may cost: most of the free memory, leaving room for the window and the rest.
/// Where the free memory can't be told (the browser) the old fixed ceiling stays.
pub fn tree_budget() -> u64 {
    const FLOOR: u64 = 1 << 30;
    const FALLBACK: u64 = 5 << 30;
    match available() {
        Some(a) => (a / 10 * 6).max(FLOOR),
        None => FALLBACK,
    }
}
