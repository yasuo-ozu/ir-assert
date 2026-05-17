mod build;
pub mod env;
mod ir;
pub mod predicate;

use env::EnvSpec;
pub use ir::{BasicBlockIr, FunctionIr};
use std::collections::HashMap;
use std::fmt::Display;

#[doc(hidden)]
pub use ir_assert_macro::__assert_ir_impl;

/// Trait for all predicate types that can be evaluated against a function's IR.
pub trait Predicate: Display {
    /// Evaluate the predicate against a function's IR.
    fn evaluate(
        &self,
        fn_name: &str,
        functions: &HashMap<String, FunctionIr>,
        env: &crate::env::EnvSpec,
    ) -> Result<(), String>;

    /// Collect all environment specifications referenced by this predicate tree.
    fn collect_environments(&self, envs: &mut Vec<crate::env::EnvSpec>) {
        envs.push(EnvSpec::DEFAULT);
    }
}

/// Assert properties of LLVM IR for given functions/closures.
///
/// Use inside `#[test]` functions:
///
/// ```rust,ignore
/// use ir_assert::assert_ir;
///
/// #[test]
/// fn test_optimized() {
///     assert_ir!(basic_blocks.len().eq(1) & calls.len().eq(0), my_fn);
/// }
/// ```
///
/// Predicate syntax is provided by the [`crate::predicate`] module.
///
/// The predicate expression can use:
/// - `basic_blocks.len().eq(1)`, `calls.len().eq(0)`, etc. with `.eq()`, `.ne()`,
///   `.lt()`, `.le()`, `.gt()`, `.ge()` comparison methods
/// - `&`, `|`, `!` — logical combinators
/// - `basic_blocks.all(|bb| ...)`, `basic_blocks.any(|bb| ...)` — quantifiers
/// - `basic_blocks.at(N).prop().len().eq(X)` — indexed block access
/// - `rustc("1.86") & ir_pred` — evaluate ir_pred against IR from rustc 1.86
/// - `target("triple") & ir_pred` — evaluate ir_pred against IR for target
///
/// Target example:
///
/// ```rust
/// use ir_assert::assert_ir;
///
/// fn add(a: u64, b: u64) -> u64 { a + b }
///
/// #[test]
/// fn test_target_specific() {
///     assert_ir!(
///         target("x86_64-unknown-linux-gnu")
///             & basic_blocks.len().eq(1),
///         add
///     );
/// }
/// ```
///
/// Target example with generic function and closure:
///
/// ```rust
/// use ir_assert::assert_ir;
///
/// fn id<T>(x: T) -> T { x }
///
/// #[test]
/// fn test_target_with_generic_and_closure() {
///     assert_ir!(
///         target("x86_64-unknown-linux-gnu")
///             & basic_blocks.len().eq(1)
///             & calls.len().eq(0),
///         id::<u64>,
///         |a: usize, b: usize| a + b
///     );
/// }
/// ```
#[macro_export]
macro_rules! assert_ir {
    ($($tt:tt)*) => {
        $crate::__assert_ir_impl!($crate, $($tt)*);
    };
}

#[doc(hidden)]
#[track_caller]
pub fn __macro_internal(
    cargo: &str,
    rustup: &str,
    manifest_dir: &str,
    crate_name: &str,
    is_test: bool,
    symbol: &str,
    pred: &dyn Predicate,
    pred_str: &str,
    target_names: &[&str],
) {
    let exe_path =
        std::env::current_exe().unwrap_or_else(|e| panic!("Cannot obtain exe path: {}", e));
    let ir_target_dir = exe_path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .unwrap()
        .join("ir-assert");

    // Synchronize: only one thread builds the IR file at a time
    let _lock = build::BUILD_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // 1. Collect all environments from the predicate tree
    let mut envs = Vec::new();
    pred.collect_environments(&mut envs);

    // Deduplicate
    envs.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
    envs.dedup();

    // 2. Build IR for all environments
    let all_env_functions = build::load_all_envs(
        cargo,
        rustup,
        manifest_dir,
        crate_name,
        is_test,
        &envs,
        &ir_target_dir,
    )
    .unwrap_or_else(|errors| {
        let mut msg = String::from("ir-assert: none of the environments are available\n");
        for (env, e) in &errors {
            msg.push_str(&format!("\n  [{}]\n", env));
            for line in e.lines() {
                msg.push_str(&format!("    {}\n", line));
            }
        }
        panic!("{}", msg);
    });

    // 3. Find the container function in default IR
    let default_functions = all_env_functions
        .values()
        .next()
        .unwrap_or_else(|| panic!("ir-assert: none of the environments are available"));
    let container = default_functions
        .iter()
        .find(|f| f.name == symbol)
        .unwrap_or_else(|| {
            panic!(
                "ir-assert: container function '{}' not found in IR. ",
                symbol,
            )
        });

    // 4. Find functions referenced by the container
    let referenced = build::find_referenced_functions(container);

    if referenced.is_empty() {
        panic!(
            "ir-assert: no function references found in container '{}'. \
             Make sure target functions are referenced via inline asm.",
            symbol
        );
    }

    // 5. For each referenced function, build per-env FunctionIr list and evaluate
    for (idx, func_name) in referenced.iter().enumerate() {
        // Check if the function exists in default IR and has blocks
        let default_func = default_functions.iter().find(|f| f.name == *func_name);
        if let Some(f) = default_func {
            if f.blocks.is_empty() {
                continue;
            }
        }

        // Build the per-env mapping for this target function
        let mut env_function_irs: Vec<(crate::env::EnvSpec, ir::FunctionIr)> = Vec::new();

        for (env, functions) in &all_env_functions {
            if let Some(env_container) = functions.iter().find(|f| f.name == symbol) {
                let env_referenced = build::find_referenced_functions(env_container);
                if let Some(env_func_name) = env_referenced.get(idx) {
                    if let Some(func) = functions.iter().find(|f| f.name == *env_func_name) {
                        if !func.blocks.is_empty() {
                            env_function_irs.push((env.clone(), func.clone()));
                        }
                    }
                }
            }
        }

        // Evaluate on each available environment.
        let mut evaluated_any = false;
        for (env, func_ir) in &env_function_irs {
            let env_functions: HashMap<String, FunctionIr> = all_env_functions
                .get(env)
                .map(|funcs| {
                    funcs
                        .iter()
                        .cloned()
                        .map(|f| (f.name.clone(), f))
                        .collect::<HashMap<_, _>>()
                })
                .unwrap_or_default();
            evaluated_any = true;
            if let Err(reason) = pred.evaluate(&func_ir.name, &env_functions, env) {
                let target_label = target_names.get(idx).copied().unwrap_or(func_name.as_str());
                let indented_reason = reason.replace('\n', "\n    ");

                let raw_ir = &func_ir.raw;

                panic!(
                    "ir-assert: assertion failed\n  \
                     predicate: {}\n  \
                     target: {}\n  \
                     environment: {}\n  \
                     reason:\n    {}\n\n{}",
                    pred_str, target_label, env, indented_reason, raw_ir
                );
            }
        }
        if !evaluated_any {
            let target_label = target_names.get(idx).copied().unwrap_or(func_name.as_str());
            let available_envs: Vec<_> = env_function_irs
                .iter()
                .map(|(e, _)| format!("{}", e))
                .collect();
            panic!(
                "ir-assert: none of the environments matched the predicate\n  \
                 predicate: {}\n  \
                 target: {}\n  \
                 available environments: [{}]",
                pred_str,
                target_label,
                available_envs.join(", ")
            );
        }
    }
}
