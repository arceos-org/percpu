use core::sync::atomic::{AtomicBool, Ordering};

use percpu_macros::percpu_symbol_vma;

/// When the `custom-base` feature is disabled, this atomic is used to mark
/// whether the per-CPU data areas have been initialized.
///
/// When the `custom-base` feature is enabled, this atomic is used as a mutex
/// and is cleared after the initialization to enable re-initialization.
static IS_INIT: AtomicBool = AtomicBool::new(false);

const fn align_up_64(val: usize) -> usize {
    const SIZE_64BIT: usize = 0x40;
    (val + SIZE_64BIT - 1) & !(SIZE_64BIT - 1)
}

/// The custom per-CPU data area base address. We only employ `AtomicUsize`'s
/// interior mutability and atomicity here.
#[cfg(any(feature = "custom-base", not(target_os = "none")))]
static PERCPU_AREA_BASE: spin::once::Once<usize> = spin::once::Once::new();

extern "C" {
    fn _percpu_start();
    fn _percpu_end();
    // WARNING: `_percpu_load_start`/`_percpu_load_end` (i.e. symbols in the
    // `.percpu` section) must be used with `percpu_symbol_vma!` macro to get
    // their VMA addresses. Casting them directly to `usize` may lead to
    // unexpected results, including:
    // - Rust assuming they are valid pointers and optimizing code based on that
    //   assumption (they are non-zero), causing unexpected runtime errors;
    // - Link-time errors because they are too far away from the program counter
    //.  (when Rust uses PC-relative addressing).
    //
    // See https://github.com/arceos-org/percpu/issues/18 for more details.
    fn _percpu_load_start();
    fn _percpu_load_end();
}

/// Returns the number of per-CPU data areas reserved in the `.percpu` section.
pub fn percpu_area_num() -> usize {
    (_percpu_end as *const () as usize - _percpu_start as *const () as usize)
        / align_up_64(percpu_area_size())
}

/// Returns the per-CPU data area size for one CPU.
pub fn percpu_area_size() -> usize {
    percpu_symbol_vma!(_percpu_load_end) - percpu_symbol_vma!(_percpu_load_start)
}

/// Returns the per-CPU data area size for the given number of CPUs.
#[cfg(feature = "custom-base")]
pub fn percpu_area_size_for_cpus(cpu_count: usize) -> usize {
    cpu_count * align_up_64(percpu_area_size())
}

/// Returns the base address of the per-CPU data area on the given CPU.
///
/// if `cpu_id` is 0, it returns the base address of all per-CPU data areas.
pub fn percpu_area_base(cpu_id: usize) -> usize {
    cfg_if::cfg_if! {
        if #[cfg(any(feature = "custom-base", not(target_os = "none")))] {
            let base = *PERCPU_AREA_BASE.get().unwrap();
        } else {
            let base = _percpu_start as *const () as usize;
        }
    }
    base + cpu_id * align_up_64(percpu_area_size())
}

/// Initialize per-CPU data areas using the `.percpu` section.
///
/// This function is for multi-core static mode (default mode). The per-CPU data
/// areas are allocated statically in the `.percpu` section by the linker script.
///
/// On bare metal, it copies the primary CPU's data to other CPUs.
/// On Linux, it allocates memory dynamically for the per-CPU data areas.
///
/// Returns the number of areas initialized. If this function has been called
/// before, it does nothing and returns 0.
#[cfg(not(feature = "custom-base"))]
pub fn init_static() -> usize {
    // Avoid re-initialization.
    if IS_INIT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return 0;
    }

    // Check if the `.percpu` section is loaded at VMA address 0 when feature "non-zero-vma" is disabled.
    //
    // `_percpu_load_start as *const () as usize` cannot be used here because
    // rust will assume a `*const ()` is a valid pointer and will not be 0,
    // causing unexpected `0 != 0` assertion failure.
    #[cfg(not(feature = "non-zero-vma"))]
    {
        assert_eq!(
            percpu_symbol_vma!(_percpu_load_start), 0,
            "The `.percpu` section must be loaded at VMA address 0 when feature \"non-zero-vma\" is disabled"
        )
    }

    // When running on Linux, we allocate the per-CPU data area here.
    #[cfg(not(target_os = "none"))]
    {
        let total_size = _percpu_end as *const () as usize - _percpu_start as *const () as usize;
        let layout = std::alloc::Layout::from_size_align(total_size, 0x1000).unwrap();
        let base = unsafe { std::alloc::alloc(layout) as usize };
        PERCPU_AREA_BASE.call_once(|| base);
    }

    // Get the number of per-CPU data areas.
    let cpu_count = percpu_area_num();

    // Copy per-cpu data of the primary CPU to other CPUs (bare metal only).
    #[cfg(target_os = "none")]
    {
        let base = percpu_area_base(0);
        let size = percpu_area_size();
        for i in 1..cpu_count {
            let secondary_base = percpu_area_base(i);
            assert!(secondary_base + size <= _percpu_end as *const () as usize);

            unsafe {
                core::ptr::copy_nonoverlapping(base as *const u8, secondary_base as *mut u8, size);
            }
        }
    }

    cpu_count
}

/// Initialize per-CPU data areas with user-provided memory.
///
/// This function is for multi-core dynamic mode (`custom-base` feature). The caller
/// is responsible for allocating memory for the per-CPU data areas.
///
/// # Arguments
/// - `base`: Base address of the user-allocated memory.
/// - `cpu_count`: Number of CPUs.
///
/// Returns the number of areas initialized. Can be called repeatedly for
/// re-initialization.
#[cfg(feature = "custom-base")]
pub fn init_dynamic(base: *const (), cpu_count: usize) -> usize {
    // Avoid re-initialization (use as mutex for custom-base mode).
    if IS_INIT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return 0;
    }

    // Check if the `.percpu` section is loaded at VMA address 0 when feature "non-zero-vma" is disabled.
    #[cfg(not(feature = "non-zero-vma"))]
    {
        assert_eq!(
            percpu_symbol_vma!(_percpu_load_start), 0,
            "The `.percpu` section must be loaded at VMA address 0 when feature \"non-zero-vma\" is disabled"
        )
    }

    // Store the user-provided base address.
    PERCPU_AREA_BASE.call_once(|| base as _);

    // Enable re-initialization for custom-base mode.
    IS_INIT.store(false, Ordering::Release);

    cpu_count
}

/// Reads the architecture-specific per-CPU data register.
///
/// This register is used to hold the per-CPU data base on each CPU.
pub fn read_percpu_reg() -> usize {
    let tp: usize;
    unsafe {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "x86_64")] {
                tp = if cfg!(target_os = "linux") {
                    SELF_PTR.read_current_raw()
                } else if cfg!(target_os = "none") {
                    x86::msr::rdmsr(x86::msr::IA32_GS_BASE) as usize
                } else {
                    unimplemented!()
                };
            } else if #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))] {
                core::arch::asm!("mv {}, gp", out(reg) tp)
            } else if #[cfg(all(target_arch = "aarch64", not(feature = "arm-el2")))] {
                core::arch::asm!("mrs {}, TPIDR_EL1", out(reg) tp)
            } else if #[cfg(all(target_arch = "aarch64", feature = "arm-el2"))] {
                core::arch::asm!("mrs {}, TPIDR_EL2", out(reg) tp)
            } else if #[cfg(target_arch = "loongarch64")] {
                // Register Convention
                // https://docs.kernel.org/arch/loongarch/introduction.html#gprs
                core::arch::asm!("move {}, $r21", out(reg) tp)
            } else if #[cfg(target_arch = "arm")] {
                core::arch::asm!("mrc p15, 0, {}, c13, c0, 3", out(reg) tp)
            }
        }
    }
    cfg_if::cfg_if! {
        if #[cfg(feature = "non-zero-vma")] {
            tp + percpu_symbol_vma!(_percpu_load_start)
        } else {
            tp
        }
    }
}

/// Writes the architecture-specific per-CPU data register.
///
/// This register is used to hold the per-CPU data base on each CPU.
///
/// # Safety
///
/// This function is unsafe because it writes the low-level register directly.
pub unsafe fn write_percpu_reg(tp: usize) {
    cfg_if::cfg_if! {
        if #[cfg(feature = "non-zero-vma")] {
            let tp = tp - percpu_symbol_vma!(_percpu_load_start);
        }
    };

    unsafe {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "x86_64")] {
                if cfg!(target_os = "linux") {
                    const ARCH_SET_GS: u32 = 0x1001;
                    const SYS_ARCH_PRCTL: u32 = 158;
                    core::arch::asm!(
                        "syscall",
                        in("eax") SYS_ARCH_PRCTL,
                        in("edi") ARCH_SET_GS,
                        in("rsi") tp,
                    );
                } else if cfg!(target_os = "none") {
                    x86::msr::wrmsr(x86::msr::IA32_GS_BASE, tp as u64);
                } else {
                    unimplemented!()
                }
                SELF_PTR.write_current_raw(tp);
            } else if #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))] {
                core::arch::asm!("mv gp, {}", in(reg) tp)
            } else if #[cfg(all(target_arch = "aarch64", not(feature = "arm-el2")))] {
                core::arch::asm!("msr TPIDR_EL1, {}", in(reg) tp)
            } else if #[cfg(all(target_arch = "aarch64", feature = "arm-el2"))] {
                core::arch::asm!("msr TPIDR_EL2, {}", in(reg) tp)
            } else if #[cfg(target_arch = "loongarch64")] {
                core::arch::asm!("move $r21, {}", in(reg) tp)
            } else if #[cfg(target_arch = "arm")] {
                core::arch::asm!("mcr p15, 0, {}, c13, c0, 3", in(reg) tp)
            }
        }
    }
}

/// Initializes the per-CPU data register.
///
/// It is equivalent to `write_percpu_reg(percpu_area_base(cpu_id))`, which set
/// the architecture-specific per-CPU data register to the base address of the
/// corresponding per-CPU data area.
///
/// `cpu_id` indicates which per-CPU data area to use.
pub fn init_percpu_reg(cpu_id: usize) {
    let tp = percpu_area_base(cpu_id);
    unsafe { write_percpu_reg(tp) }
}

/// To use `percpu::__priv::NoPreemptGuard::new()` and `percpu::percpu_area_base()` in macro expansion.
#[allow(unused_imports)]
use crate as percpu;

/// On x86, we use `gs:SELF_PTR` to store the address of the per-CPU data area base.
#[cfg(target_arch = "x86_64")]
#[no_mangle]
#[percpu_macros::def_percpu]
static SELF_PTR: usize = 0;
