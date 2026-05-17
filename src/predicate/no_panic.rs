use std::{collections::HashMap, fmt};

use crate::env::EnvSpec;
use crate::predicate::combinators::get_ir;
use crate::{FunctionIr, Predicate};

/// Predicate that asserts no panic-related functions are reachable from the target.
pub struct NoPanic;

fn is_panic_function(name: &str) -> bool {
    // Both Itanium and v0 mangling contain these substrings:
    // "core::panicking::*" -> contains "4core9panicking"
    // "std::panicking::*"  -> contains "3std9panicking"
    //
    // Some panic sinks can also appear as external symbols without bodies in
    // the current crate IR (for example, Option::unwrap failure paths).
    // Match those by symbol fragments as well.
    name.contains("4core9panicking")
        || name.contains("3std9panicking")
        || name.contains("unwrap_failed")
        || name.contains("expect_failed")
        || name.contains("assert_failed")
        || name.contains("slice_error_fail")
        || name.contains("panic_fmt")
        || name.contains("begin_panic")
}

fn find_panic_chain(
    fn_name: &str,
    functions: &HashMap<String, FunctionIr>,
    visited: &mut std::collections::HashSet<String>,
) -> Option<(Vec<String>, &'static str)> {
    if !visited.insert(fn_name.to_string()) {
        return None;
    }
    let Some(ir) = functions.get(fn_name) else {
        panic!("function '{}' not found in environment IR map", fn_name);
    };
    for block in &ir.blocks {
        for call in &block.calls {
            if is_panic_function(call) {
                return Some((
                    vec![fn_name.to_string(), call.clone()],
                    "panic function reachable",
                ));
            }
            if let Some((mut chain, reason)) = find_panic_chain(call, functions, visited) {
                chain.insert(0, fn_name.to_string());
                return Some((chain, reason));
            }
        }
    }
    None
}

impl Predicate for NoPanic {
    fn collect_environments(&self, envs: &mut Vec<EnvSpec>) {
        envs.push(EnvSpec::release());
    }

    fn evaluate(
        &self,
        fn_name: &str,
        functions: &HashMap<String, FunctionIr>,
        _env: &EnvSpec,
    ) -> Result<(), String> {
        let mut visited = std::collections::HashSet::new();
        visited.insert(fn_name.to_string());
        let ir = get_ir(fn_name, functions);
        for block in &ir.blocks {
            for call in &block.calls {
                if is_panic_function(call) {
                    return Err(format!(
                        "no_panic: {}\n  call chain: {} → {}",
                        "panic function reachable", fn_name, call
                    ));
                }
                if let Some((chain, reason)) = find_panic_chain(call, functions, &mut visited) {
                    return Err(format!(
                        "no_panic: {}\n  call chain: {} → {}",
                        reason,
                        fn_name,
                        chain.join(" → ")
                    ));
                }
            }
        }
        Ok(())
    }
}

impl fmt::Display for NoPanic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no_panic")
    }
}

#[allow(non_upper_case_globals)]
pub const no_panic: NoPanic = NoPanic;

impl<Rhs: crate::Predicate> ::std::ops::BitAnd<Rhs> for NoPanic {
    type Output = crate::predicate::combinators::And<NoPanic, Rhs>;
    fn bitand(self, rhs: Rhs) -> Self::Output {
        crate::predicate::combinators::And(self, rhs)
    }
}

impl<Rhs: crate::Predicate> ::std::ops::BitOr<Rhs> for NoPanic {
    type Output = crate::predicate::combinators::Or<NoPanic, Rhs>;
    fn bitor(self, rhs: Rhs) -> Self::Output {
        crate::predicate::combinators::Or(self, rhs)
    }
}

impl ::std::ops::Not for NoPanic {
    type Output = crate::predicate::combinators::Not<NoPanic>;
    fn not(self) -> Self::Output {
        crate::predicate::combinators::Not(self)
    }
}
