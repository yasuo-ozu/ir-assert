use crate::{FunctionIr, Predicate};
use core::fmt;
use std::collections::HashMap;

/// Optimization level for IR generation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum OptLevel {
    /// No preference — merges with any other variant.
    None,
    /// Build without `--release` (debug profile).
    Debug,
    /// Build with `--release` (default opt-level).
    Release,
    /// Build with `--release -C opt-level=X`.
    OptLevel(&'static str),
}

/// Specifies an environment (toolchain + target + opt level) for IR generation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EnvSpec {
    pub rustc: Option<&'static str>,
    pub target: Option<&'static str>,
    pub opt_level: OptLevel,
}

impl EnvSpec {
    pub const DEFAULT: Self = Self {
        rustc: None,
        target: None,
        opt_level: OptLevel::None,
    };

    pub const fn rustc(version: &'static str) -> Self {
        Self {
            rustc: Some(version),
            target: None,
            opt_level: OptLevel::None,
        }
    }

    pub const fn target(target: &'static str) -> Self {
        Self {
            rustc: None,
            target: Some(target),
            opt_level: OptLevel::None,
        }
    }

    pub const fn rustc_target(version: &'static str, target: &'static str) -> Self {
        Self {
            rustc: Some(version),
            target: Some(target),
            opt_level: OptLevel::None,
        }
    }

    pub const fn opt_level(level: &'static str) -> Self {
        Self {
            rustc: None,
            target: None,
            opt_level: OptLevel::OptLevel(level),
        }
    }

    pub const fn target_opt_level(target: &'static str, level: &'static str) -> Self {
        Self {
            rustc: None,
            target: Some(target),
            opt_level: OptLevel::OptLevel(level),
        }
    }

    pub const fn debug() -> Self {
        Self {
            rustc: None,
            target: None,
            opt_level: OptLevel::Debug,
        }
    }

    pub const fn release() -> Self {
        Self {
            rustc: None,
            target: None,
            opt_level: OptLevel::Release,
        }
    }

    /// Decompose this `EnvSpec` into `(version, target, opt_level)` components.
    pub fn decompose(&self) -> (Option<&str>, Option<&str>, &OptLevel) {
        (self.rustc, self.target, &self.opt_level)
    }

    /// Merge two environment specs with logical-AND semantics.
    ///
    /// Panics when constraints conflict.
    pub fn and(&self, rhs: &Self) -> Self {
        fn merge_and_field(
            lhs: Option<&'static str>,
            rhs: Option<&'static str>,
            name: &str,
        ) -> Option<&'static str> {
            match (lhs, rhs) {
                (Some(l), Some(r)) if l == r => Some(l),
                (Some(_), Some(_)) => panic!("EnvSpec::and conflict on field `{name}`"),
                (Some(v), None) | (None, Some(v)) => Some(v),
                (None, None) => None,
            }
        }

        let opt_level = match (&self.opt_level, &rhs.opt_level) {
            (OptLevel::None, other) | (other, OptLevel::None) => other.clone(),
            (OptLevel::Debug, OptLevel::Debug) => OptLevel::Debug,
            (OptLevel::Release, OptLevel::Release) => OptLevel::Release,
            (OptLevel::Release, OptLevel::OptLevel(x))
            | (OptLevel::OptLevel(x), OptLevel::Release) => OptLevel::OptLevel(x),
            (OptLevel::OptLevel(x), OptLevel::OptLevel(y)) if x == y => OptLevel::OptLevel(x),
            (l, r) => panic!(
                "EnvSpec::and conflict on field `opt_level`: {:?} vs {:?}",
                l, r
            ),
        };

        Self {
            rustc: merge_and_field(self.rustc, rhs.rustc, "rustc"),
            target: merge_and_field(self.target, rhs.target, "target"),
            opt_level,
        }
    }

    /// Merge two environment specs with logical-OR semantics.
    ///
    /// Returns a minimized union as one or two specs.
    pub fn or(&self, rhs: &Self) -> Vec<Self> {
        fn opt_subsumes(lhs: &OptLevel, rhs: &OptLevel) -> bool {
            match (lhs, rhs) {
                (OptLevel::None, _) => true,
                (l, r) => l == r,
            }
        }

        fn subsumes(lhs: &EnvSpec, rhs: &EnvSpec) -> bool {
            lhs.rustc.map_or(true, |v| rhs.rustc == Some(v))
                && lhs.target.map_or(true, |v| rhs.target == Some(v))
                && opt_subsumes(&lhs.opt_level, &rhs.opt_level)
        }

        if self == rhs {
            return vec![self.clone()];
        }
        if subsumes(self, rhs) {
            return vec![self.clone()];
        }
        if subsumes(rhs, self) {
            return vec![rhs.clone()];
        }
        vec![self.clone(), rhs.clone()]
    }
}

impl fmt::Display for EnvSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let opt_str = match &self.opt_level {
            OptLevel::None => Option::<String>::None,
            OptLevel::Debug => Some("debug".to_string()),
            OptLevel::Release => Some("release".to_string()),
            OptLevel::OptLevel(level) => Some(format!("opt_level(\"{}\")", level)),
        };

        match (self.rustc, self.target, &opt_str) {
            (None, None, None) => write!(f, "default"),
            (Some(v), None, None) => write!(f, "rustc(\"{}\")", v),
            (None, Some(t), None) => write!(f, "target(\"{}\")", t),
            (Some(v), Some(t), None) => write!(f, "rustc(\"{}\") && target(\"{}\")", v, t),
            (None, None, Some(o)) => write!(f, "{}", o),
            (Some(v), None, Some(o)) => write!(f, "rustc(\"{}\") && {}", v, o),
            (None, Some(t), Some(o)) => write!(f, "target(\"{}\") && {}", t, o),
            (Some(v), Some(t), Some(o)) => {
                write!(f, "rustc(\"{}\") && target(\"{}\") && {}", v, t, o)
            }
        }
    }
}

impl<Rhs: crate::Predicate> ::std::ops::BitAnd<Rhs> for EnvSpec {
    type Output = crate::predicate::combinators::And<EnvSpec, Rhs>;
    fn bitand(self, rhs: Rhs) -> Self::Output {
        crate::predicate::combinators::And(self, rhs)
    }
}

impl<Rhs: crate::Predicate> ::std::ops::BitOr<Rhs> for EnvSpec {
    type Output = crate::predicate::combinators::Or<EnvSpec, Rhs>;
    fn bitor(self, rhs: Rhs) -> Self::Output {
        crate::predicate::combinators::Or(self, rhs)
    }
}

impl ::std::ops::Not for EnvSpec {
    type Output = crate::predicate::combinators::Not<EnvSpec>;
    fn not(self) -> Self::Output {
        crate::predicate::combinators::Not(self)
    }
}

impl Predicate for EnvSpec {
    fn evaluate(
        &self,
        _fn_name: &str,
        _functions: &HashMap<String, FunctionIr>,
        env: &EnvSpec,
    ) -> Result<(), String> {
        let rustc_ok = self.rustc.map_or(true, |v| env.rustc == Some(v));
        let target_ok = self.target.map_or(true, |t| env.target == Some(t));
        let opt_ok = match &self.opt_level {
            OptLevel::None => true,
            other => &env.opt_level == other,
        };
        if rustc_ok && target_ok && opt_ok {
            Ok(())
        } else {
            Err(format!(
                "environment mismatch: expected {}, got {}",
                self, env
            ))
        }
    }

    fn collect_environments(&self, envs: &mut Vec<EnvSpec>) {
        envs.push(self.clone());
    }
}
