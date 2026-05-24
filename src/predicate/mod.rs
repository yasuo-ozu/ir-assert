#![allow(clippy::test_attr_in_doctest)]
//! Predicate DSL for [`assert_ir!`](crate::assert_ir).
//!
//! This module provides:
//! - function-level IR properties (for example, `basic_blocks.len().eq(1)`)
//! - block-level predicates (for example, `basic_blocks.at(0).calls().len().eq(0)`)
//! - environment predicates (for example, `target("x86_64-unknown-linux-gnu")`, `rustc("1.90")`, `opt3`)
//! - logical composition with `&`, `|`, and `!`
//!
//! Typical usage:
//!
//! ```rust
//! use ir_assert::assert_ir;
//!
//! fn add<T: std::ops::Add<Output = T>>(a: T, b: T) -> T { a + b }
//!
//! #[test]
//! fn test_ir_shape() {
//!     assert_ir!(
//!         basic_blocks.len().eq(1) & calls.len().eq(0),
//!         add::<u64>
//!     );
//! }
//! ```
//!
//! Target-specific usage:
//!
//! ```rust
//! use ir_assert::assert_ir;
//!
//! fn identity(x: u64) -> u64 { x }
//!
//! #[test]
//! fn test_target_specific() {
//!     assert_ir!(
//!         target("x86_64-unknown-linux-gnu")
//!             & basic_blocks.len().eq(1),
//!         identity
//!     );
//! }
//! ```
//!
//! Environment helpers are available directly from this module via re-exports
//! (for example, `target("...")`, `opt3`).
pub(crate) mod combinators;
mod debug;
mod dsl;
mod no_panic;

// Re-export DSL entry points for `use crate::predicate::*`.
/// Predicate that matches debug-mode IR builds.
///
/// Useful for writing conditional assertions such as:
/// `debug | calls.len().eq(0)`.
pub use debug::debug;
/// Predicate that asserts panic-related functions are not reachable
/// from the target function's call graph.
///
/// Example: `no_panic & calls.len().eq(0)`.
pub use no_panic::no_panic;

// --- DSL namespace constants ---

#[allow(non_upper_case_globals)]
/// Function-level/basic-block entry point.
///
/// Examples:
/// - `basic_blocks.len().eq(1)`
/// - `basic_blocks.at(0).calls().len().eq(0)`
/// - `basic_blocks.all(|bb| bb.instructions.len().ge(1))`
pub const basic_blocks: dsl::BasicBlocks = dsl::BasicBlocks;
#[allow(non_upper_case_globals)]
/// Function-level call-count property.
///
/// Example: `calls.len().eq(0)`.
pub const calls: dsl::PropertyAccess = dsl::PropertyAccess(dsl::Property::CallsLen);
#[allow(non_upper_case_globals)]
/// Function-level instruction-count property.
///
/// Example: `instructions.len().le(10)`.
pub const instructions: dsl::PropertyAccess = dsl::PropertyAccess(dsl::Property::InstructionsLen);
#[allow(non_upper_case_globals)]
/// Function-level alloca-count property.
///
/// Example: `allocas.len().eq(0)`.
pub const allocas: dsl::PropertyAccess = dsl::PropertyAccess(dsl::Property::AllocasLen);
#[allow(non_upper_case_globals)]
/// Function-level branch/switch-count property.
///
/// Example: `branches.len().ge(1)`.
pub const branches: dsl::PropertyAccess = dsl::PropertyAccess(dsl::Property::BranchesLen);
#[allow(non_upper_case_globals)]
/// Function-level phi-node-count property.
///
/// Example: `phi_nodes.len().eq(0)`.
pub const phi_nodes: dsl::PropertyAccess = dsl::PropertyAccess(dsl::Property::PhiNodesLen);

// --- Shared primitives ---

macro_rules! define_cmp_methods {
    ($out:ty, |$self_:ident, $op:ident| $body:expr) => {
        pub fn eq($self_, n: usize) -> $out {
            let $op = $crate::predicate::combinators::CmpOp::Eq(n);
            $body
        }
        pub fn ne($self_, n: usize) -> $out {
            let $op = $crate::predicate::combinators::CmpOp::Ne(n);
            $body
        }
        pub fn lt($self_, n: usize) -> $out {
            let $op = $crate::predicate::combinators::CmpOp::Lt(n);
            $body
        }
        pub fn le($self_, n: usize) -> $out {
            let $op = $crate::predicate::combinators::CmpOp::Le(n);
            $body
        }
        pub fn gt($self_, n: usize) -> $out {
            let $op = $crate::predicate::combinators::CmpOp::Gt(n);
            $body
        }
        pub fn ge($self_, n: usize) -> $out {
            let $op = $crate::predicate::combinators::CmpOp::Ge(n);
            $body
        }
    };
}

pub(crate) use define_cmp_methods;

// --- Environment helpers ---

use crate::env::EnvSpec;

/// Returns a predicate that selects a specific rustc toolchain version.
/// When combined with IR predicates, the IR will be generated using that toolchain.
///
/// ```
/// # use ir_assert::assert_ir;
/// # fn my_fn() {}
/// assert_ir!(rustc("1.80") & basic_blocks.len().eq(1), my_fn);
/// ```
#[allow(non_snake_case)]
pub const fn rustc(version: &'static str) -> EnvSpec {
    EnvSpec::rustc(version)
}

/// Returns a predicate that selects a specific target triple.
/// When combined with IR predicates, the IR will be generated for that target.
///
/// ```
/// # use ir_assert::assert_ir;
/// # fn my_fn() {}
/// assert_ir!(target("wasm32-unknown-unknown") & basic_blocks.len().eq(1), my_fn);
/// ```
pub const fn target(triple: &'static str) -> EnvSpec {
    EnvSpec::target(triple)
}

#[allow(non_upper_case_globals)]
/// Select the `wasm32-unknown-unknown` target environment.
pub const target_wasm32_unknown_unknown: EnvSpec = target("wasm32-unknown-unknown");
#[allow(non_upper_case_globals)]
/// Select the `x86_64-unknown-linux-gnu` target environment.
pub const target_x86_64_unknown_linux_gnu: EnvSpec = target("x86_64-unknown-linux-gnu");
#[allow(non_upper_case_globals)]
/// Select the `x86_64-apple-darwin` target environment.
pub const target_x86_64_apple_darwin: EnvSpec = target("x86_64-apple-darwin");
#[allow(non_upper_case_globals)]
/// Select the `aarch64-apple-darwin` target environment.
pub const target_aarch64_apple_darwin: EnvSpec = target("aarch64-apple-darwin");
#[allow(non_upper_case_globals)]
/// Select the `aarch64-unknown-linux-gnu` target environment.
pub const target_aarch64_unknown_linux_gnu: EnvSpec = target("aarch64-unknown-linux-gnu");
#[allow(non_upper_case_globals)]
/// Select the `x86_64-pc-windows-msvc` target environment.
pub const target_x86_64_pc_windows_msvc: EnvSpec = target("x86_64-pc-windows-msvc");
#[allow(non_upper_case_globals)]
/// Select the `aarch64-pc-windows-msvc` target environment.
pub const target_aarch64_pc_windows_msvc: EnvSpec = target("aarch64-pc-windows-msvc");

const fn opt_level(level: &'static str) -> EnvSpec {
    EnvSpec::opt_level(level)
}

#[allow(non_upper_case_globals)]
/// Select optimization level `0`.
pub const opt0: EnvSpec = opt_level("0");
#[allow(non_upper_case_globals)]
/// Select optimization level `1`.
pub const opt1: EnvSpec = opt_level("1");
#[allow(non_upper_case_globals)]
/// Select optimization level `2`.
pub const opt2: EnvSpec = opt_level("2");
#[allow(non_upper_case_globals)]
/// Select optimization level `3`.
pub const opt3: EnvSpec = opt_level("3");
#[allow(non_upper_case_globals)]
/// Select optimization level `s` (optimize for size).
pub const opt_s: EnvSpec = opt_level("s");
#[allow(non_upper_case_globals)]
/// Select optimization level `z` (aggressively optimize for size).
pub const opt_z: EnvSpec = opt_level("z");
