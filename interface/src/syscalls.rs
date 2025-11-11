//! Solana's low-level C syscalls provided by the SVM runtime.
//!
//! ---
//! Copied from `pinocchio` – commit bde84880a6709bbda4da8767b5f0a42d9678d07c
//!
//! Modifications made:
//! - Changed some formatting
//! - Changed function argument names to match the rust [`core::ptr`] calls.
//! - Added fallbacks for non-solana targets; i.e., when `#[cfg(not(target_os = "solana"))]`.
//!
//! Original: <https://github.com/anza-xyz/pinocchio/blob/bde84880a6709bbda4da8767b5f0a42d9678d07c/sdk/log/crate/src/logger.rs>

#[cfg(all(target_os = "solana", not(target_feature = "static-syscalls")))]
mod inner {
    // Syscalls provided by the SVM runtime (SBPFv0, SBPFv1 and SBPFv2).
    extern "C" {
        pub fn sol_log_(message: *const u8, len: u64);

        pub fn sol_memcpy_(dst: *mut u8, src: *const u8, count: u64);

        pub fn sol_memset_(dst: *mut u8, val: u8, count: u64);

        pub fn sol_remaining_compute_units() -> u64;
    }
}

#[cfg(all(target_os = "solana", target_feature = "static-syscalls"))]
mod inner {
    // Syscalls provided by the SVM runtime (SBPFv3 and newer).
    pub(crate) fn sol_log_(message: *const u8, length: u64) {
        // murmur32 hash of "sol_log_"
        let syscall: extern "C" fn(*const u8, u64) = unsafe { core::mem::transmute(544561597u64) };
        syscall(message, length)
    }

    pub(crate) fn sol_memcpy_(dst: *mut u8, src: *const u8, count: u64) {
        // murmur32 hash of "sol_memcpy_"
        let syscall: extern "C" fn(*mut u8, *const u8, u64) =
            unsafe { core::mem::transmute(1904002211u64) };
        syscall(dst, src, count)
    }

    pub(crate) fn sol_memset_(dst: *mut u8, val: u8, count: u64) {
        // murmur32 hash of "sol_memset_"
        let syscall: extern "C" fn(*mut u8, u8, u64) =
            unsafe { core::mem::transmute(930151202u64) };
        syscall(dst, val, count)
    }

    pub(crate) fn sol_remaining_compute_units() -> u64 {
        // murmur32 hash of "sol_remaining_compute_units"
        let syscall: extern "C" fn() -> u64 = unsafe { core::mem::transmute(3991886574u64) };
        syscall()
    }
}

#[cfg(not(target_os = "solana"))]
#[allow(dead_code)]
mod inner {
    pub(crate) fn sol_log_(_message: *const u8, _length: u64) {}

    /// Copies `count` bytes from `src` to `dst`.
    ///
    /// This function is not marked as `unsafe` so its function signature matches the other syscall
    /// implementations.
    ///
    /// However, caller should adhere to the safety contract in [`core::ptr::copy_nonoverlapping`].
    pub(crate) fn sol_memcpy_(dst: *mut u8, src: *const u8, count: u64) {
        unsafe {
            core::ptr::copy_nonoverlapping(src, dst, count as usize);
        }
    }

    /// Sets `count` bytes of memory to `val`, starting at `dst`.
    ///
    /// This function is not marked as `unsafe` so its function signature matches the other syscall
    /// implementations.
    ///
    /// However, caller should adhere to the safety contract in [`core::ptr::write_bytes`].
    pub(crate) fn sol_memset_(dst: *mut u8, val: u8, count: u64) {
        unsafe {
            core::ptr::write_bytes(dst, val, count as usize);
        }
    }

    pub(crate) fn sol_remaining_compute_units() -> u64 {
        0
    }
}

#[allow(unused_imports)]
use inner::*;
