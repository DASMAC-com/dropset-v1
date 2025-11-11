//! Solana's low-level C syscalls provided by the SVM runtime.
//!
//! ---
//! Copied from `pinocchio` – commit bde84880a6709bbda4da8767b5f0a42d9678d07c
//!
//! Modifications made:
//! - Removed unused syscalls
//! - Changed some formatting
//! - Changed function argument names to match the rust [`core::ptr`] calls.
//! - Added fallbacks for non-solana targets; i.e., when `#[cfg(not(target_os = "solana"))]`.
//!
//! Original: <https://github.com/anza-xyz/pinocchio/blob/bde84880a6709bbda4da8767b5f0a42d9678d07c/sdk/log/crate/src/logger.rs>

#[cfg(all(target_os = "solana", not(target_feature = "static-syscalls")))]
/// Syscalls provided by the SVM runtime (SBPFv0, SBPFv1 and SBPFv2).
mod inner {
    extern "C" {
        pub fn sol_memcpy_(dst: *mut u8, src: *const u8, count: u64);

        pub fn sol_memset_(dst: *mut u8, val: u8, count: u64);
    }
}

#[cfg(all(target_os = "solana", target_feature = "static-syscalls"))]
/// Syscalls provided by the SVM runtime (SBPFv3 and newer).
mod inner {
    pub(crate) unsafe fn sol_memcpy_(dst: *mut u8, src: *const u8, count: u64) {
        // murmur32 hash of "sol_memcpy_"
        let syscall: extern "C" fn(*mut u8, *const u8, u64) =
            unsafe { core::mem::transmute(1904002211u64) };
        syscall(dst, src, count)
    }

    pub(crate) unsafe fn sol_memset_(dst: *mut u8, val: u8, count: u64) {
        // murmur32 hash of "sol_memset_"
        let syscall: extern "C" fn(*mut u8, u8, u64) =
            unsafe { core::mem::transmute(930151202u64) };
        syscall(dst, val, count)
    }
}

#[cfg(not(target_os = "solana"))]
#[allow(dead_code)]
/// Syscall fallbacks for non-`solana` targets.
mod inner {
    /// Copies `count` bytes from `src` to `dst`.
    ///
    /// # Safety
    ///
    /// Caller should adhere to the safety contract in [`core::ptr::copy_nonoverlapping`].
    pub(crate) unsafe fn sol_memcpy_(dst: *mut u8, src: *const u8, count: u64) {
        unsafe {
            core::ptr::copy_nonoverlapping(src, dst, count as usize);
        }
    }

    /// Sets `count` bytes of memory to `val`, starting at `dst`.
    ///
    /// # Safety
    ///
    /// However, caller should adhere to the safety contract in [`core::ptr::write_bytes`].
    pub(crate) unsafe fn sol_memset_(dst: *mut u8, val: u8, count: u64) {
        unsafe {
            core::ptr::write_bytes(dst, val, count as usize);
        }
    }
}

#[allow(unused_imports)]
pub(crate) use inner::*;
