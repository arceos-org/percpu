//! Naive implementation for single CPU use.

/// Returns the per-CPU data area size for one CPU.
///
/// Always returns `0` for "sp-naive" use.
pub fn percpu_area_size() -> usize {
    0
}

/// Returns the number of per-CPU data areas reserved.
///
/// Always returns `1` for "sp-naive" use.
pub fn percpu_area_num() -> usize {
    1
}

/// Returns the base address of the per-CPU data area on the given CPU.
///
/// Always returns `0` for "sp-naive" use.
pub fn percpu_area_base(_cpu_id: usize) -> usize {
    0
}

/// Reads the architecture-specific per-CPU data register.
///
/// Always returns `0` for "sp-naive" use.
pub fn read_percpu_reg() -> usize {
    0
}

/// Writes the architecture-specific per-CPU data register.
///
/// No effect for "sp-naive" use.
///
/// # Safety
///
/// This function is marked as `unsafe` for consistency with non "sp-naive"
/// implementations.
pub unsafe fn write_percpu_reg(_tp: usize) {}

/// Initializes the per-CPU data register.
///
/// No effect for "sp-naive" use.
pub fn init_percpu_reg(_cpu_id: usize) {}

/// Initialize all per-CPU data areas.
///
/// Returns the number of areas initialized.
///
/// For "sp-naive" use it does nothing and returns `1`.
///
/// When the "custom-base" feature is enabled, this function accepts a base address
/// and CPU count, but they are ignored since "sp-naive" uses global variables.
#[cfg(not(feature = "custom-base"))]
pub fn init() -> usize {
    1
}

/// Initialize all per-CPU data areas.
///
/// Returns the number of areas initialized.
///
/// For "sp-naive" use it does nothing and returns `1`.
///
/// The `base` and `cpu_count` parameters are ignored since "sp-naive" uses global
/// variables instead of per-CPU memory areas. This signature is provided for API
/// compatibility with the "custom-base" feature.
#[cfg(feature = "custom-base")]
pub fn init(_base: *const (), _cpu_count: usize) -> usize {
    1
}

/// Returns the per-CPU data area size for the given number of CPUs.
///
/// Always returns `0` for "sp-naive" use since no per-CPU memory areas are needed.
///
/// This function is provided for API compatibility with the "custom-base" feature.
#[cfg(feature = "custom-base")]
pub fn percpu_area_size_for_cpus(_cpu_count: usize) -> usize {
    0
}
