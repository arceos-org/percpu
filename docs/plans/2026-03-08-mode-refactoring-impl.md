# Implementation Plan: Mode Refactoring

## Overview

This plan implements the mode refactoring design to split `init()` into `init_static()` and `init_dynamic()`.

## Prerequisites

- [x] Design document approved: `docs/plans/2026-03-08-mode-refactoring-design.md`

## Implementation Steps

### Step 1: Update `naive.rs` (sp-naive mode)

**File**: `percpu/src/naive.rs`

**Changes**:
1. Rename `init()` to `init_static()`
2. Remove `#[cfg(feature = "custom-base")]` conditional `init()` function
3. Remove `#[cfg(feature = "custom-base")]` `percpu_area_size_for_cpus()` function

**Verification**:
```bash
cargo check -p percpu --features sp-naive
```

---

### Step 2: Update `imp.rs` (multi-core modes)

**File**: `percpu/src/imp.rs`

**Changes**:
1. Rename `init()` to `init_static()` for non-custom-base mode
2. Rename `init()` to `init_dynamic()` for custom-base mode
3. Use `#[cfg(not(feature = "custom-base"))]` for `init_static()`
4. Use `#[cfg(feature = "custom-base")]` for `init_dynamic()`

**Verification**:
```bash
cargo check -p percpu
cargo check -p percpu --features custom-base
```

---

### Step 3: Update `lib.rs` exports

**File**: `percpu/src/lib.rs`

**Changes**:
1. Keep `pub use self::imp::*;` for most exports
2. No additional changes needed since functions are exported via `imp::*`

**Verification**:
```bash
cargo check -p percpu --features sp-naive
cargo check -p percpu
cargo check -p percpu --features custom-base
```

---

### Step 4: Update tests

**File**: `percpu/tests/test_percpu.rs`

**Changes**:
1. Replace `init()` with `init_static()` in sp-naive section (line 50)
2. Replace `init()` with `init_static()` in default mode section (line 56)
3. Replace `init(base, cpu_count)` with `init_dynamic(base, cpu_count)` in custom-base section (line 73)
4. Remove the `#[cfg(feature = "custom-base")] init(...)` from sp-naive section (line 47-48) since sp-naive + custom-base is no longer a valid combination

**Verification**:
```bash
cargo test -p percpu --features sp-naive
cargo test -p percpu --features non-zero-vma
cargo test -p percpu --features custom-base,non-zero-vma
```

---

### Step 5: Update README.md

**File**: `percpu/README.md`

**Changes**:
1. Update example code to use `init_static()` and `init_dynamic()`
2. Update Working Modes table to reflect new API
3. Update Cargo Features section to clarify mode vs auxiliary features
4. Update Default values section if needed

**Verification**:
```bash
cargo test -p percpu --doc --features sp-naive
cargo test -p percpu --doc --features non-zero-vma
```

---

### Step 6: Update version number

**Files**: `Cargo.toml` (workspace), `percpu/Cargo.toml`, `percpu_macros/Cargo.toml`

**Changes**:
1. Update version from `0.2.2` to `0.3.0`

**Verification**:
```bash
cargo check --workspace
```

---

### Step 7: Run full test suite

**Commands**:
```bash
# Test all mode combinations
cargo test -p percpu --features sp-naive
cargo test -p percpu --features "sp-naive,non-zero-vma"
cargo test -p percpu --features non-zero-vma
cargo test -p percpu --features "custom-base,non-zero-vma"

# Test macros
cargo test -p percpu_macros

# Doc tests
cargo test -p percpu --doc --features sp-naive
cargo test -p percpu --doc --features non-zero-vma
```

---

### Step 8: Commit changes

**Commands**:
```bash
git add -A
git commit -m "feat!: split init() into init_static() and init_dynamic()

BREAKING CHANGE: init() is replaced by init_static() (for sp-naive and
default modes) and init_dynamic() (for custom-base mode).

- Rename init() to init_static() in sp-naive and default modes
- Rename init() to init_dynamic() in custom-base mode
- Update tests and documentation
- Bump version to 0.3.0"
```

---

## Rollback Plan

If issues are found:
1. `git revert HEAD` to undo the commit
2. Or use `git reset --hard HEAD~1` if not pushed

## Success Criteria

- [ ] All tests pass for all mode combinations
- [ ] Documentation is updated and accurate
- [ ] Version bumped to 0.3.0
- [ ] Changes committed to git