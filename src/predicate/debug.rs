use std::{collections::HashMap, fmt};

use crate::env::{EnvSpec, OptLevel};
use crate::{FunctionIr, Predicate};

/// Environment predicate that builds IR in debug mode (without `--release`).
///
/// In debug mode, `debug_assert!` is present, integer overflow panics, etc.
/// Combine with IR predicates to verify debug-mode behavior:
///
/// ```rust,ignore
/// // debug_assert! is active in debug builds, so panic is reachable
/// assert_ir!(debug & !no_panic, my_fn_with_debug_assert);
/// ```
pub struct Debug;

impl Predicate for Debug {
    fn evaluate(
        &self,
        _fn_name: &str,
        _functions: &HashMap<String, FunctionIr>,
        env: &EnvSpec,
    ) -> Result<(), String> {
        if env.opt_level == OptLevel::Debug {
            Ok(())
        } else {
            Err(format!("expected debug build, got {}", env))
        }
    }

    fn collect_environments(&self, envs: &mut Vec<EnvSpec>) {
        envs.push(EnvSpec::debug());
    }
}

impl fmt::Display for Debug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "debug")
    }
}

#[allow(non_upper_case_globals)]
pub const debug: Debug = Debug;

impl<Rhs: crate::Predicate> ::std::ops::BitAnd<Rhs> for Debug {
    type Output = crate::predicate::combinators::And<Debug, Rhs>;
    fn bitand(self, rhs: Rhs) -> Self::Output {
        crate::predicate::combinators::And(self, rhs)
    }
}

impl<Rhs: crate::Predicate> ::std::ops::BitOr<Rhs> for Debug {
    type Output = crate::predicate::combinators::Or<Debug, Rhs>;
    fn bitor(self, rhs: Rhs) -> Self::Output {
        crate::predicate::combinators::Or(self, rhs)
    }
}

impl ::std::ops::Not for Debug {
    type Output = crate::predicate::combinators::Not<Debug>;
    fn not(self) -> Self::Output {
        crate::predicate::combinators::Not(self)
    }
}
