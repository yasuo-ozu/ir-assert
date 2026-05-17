use std::{collections::HashMap, fmt};

pub use super::dsl::Property;
use crate::env::EnvSpec;
use crate::{FunctionIr, Predicate};

#[derive(Debug, Clone, Copy)]
pub enum CmpOp {
    Eq(usize),
    Ne(usize),
    Lt(usize),
    Le(usize),
    Gt(usize),
    Ge(usize),
}

impl fmt::Display for CmpOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            CmpOp::Eq(n) => write!(f, "eq({})", n),
            CmpOp::Ne(n) => write!(f, "ne({})", n),
            CmpOp::Lt(n) => write!(f, "lt({})", n),
            CmpOp::Le(n) => write!(f, "le({})", n),
            CmpOp::Gt(n) => write!(f, "gt({})", n),
            CmpOp::Ge(n) => write!(f, "ge({})", n),
        }
    }
}

impl CmpOp {
    fn check(&self, actual: usize, prop_name: &str) -> Result<(), String> {
        let (pass, sym, n) = match *self {
            CmpOp::Eq(n) => (actual == n, "==", n),
            CmpOp::Ne(n) => (actual != n, "!=", n),
            CmpOp::Lt(n) => (actual < n, "<", n),
            CmpOp::Le(n) => (actual <= n, "<=", n),
            CmpOp::Gt(n) => (actual > n, ">", n),
            CmpOp::Ge(n) => (actual >= n, ">=", n),
        };
        if pass {
            Ok(())
        } else {
            Err(format!(
                "{}: expected {} {}, got {}",
                prop_name, sym, n, actual
            ))
        }
    }
}

pub fn get_ir<'a>(fn_name: &str, functions: &'a HashMap<String, FunctionIr>) -> &'a FunctionIr {
    functions
        .get(fn_name)
        .unwrap_or_else(|| panic!("function '{}' not found in environment IR map", fn_name))
}

// --- BlockPred (kept as enum for dynamic closure returns) ---

pub enum BlockPred {
    Cmp { property: Property, op: CmpOp },
    And(Box<BlockPred>, Box<BlockPred>),
    Or(Box<BlockPred>, Box<BlockPred>),
    Not(Box<BlockPred>),
}

impl BlockPred {
    pub fn evaluate_block(&self, f: &FunctionIr, block_idx: usize) -> Result<(), String> {
        match self {
            BlockPred::Cmp { property, op } => op.check(
                f.compute_block_property(block_idx, *property),
                property.name(),
            ),
            BlockPred::And(l, r) => merge_results(
                l.evaluate_block(f, block_idx),
                r.evaluate_block(f, block_idx),
            ),
            BlockPred::Or(l, r) => or_results(l.evaluate_block(f, block_idx), || {
                r.evaluate_block(f, block_idx)
            }),
            BlockPred::Not(p) => negate_result(p.evaluate_block(f, block_idx)),
        }
    }

    fn chain_cmp(self, op: CmpOp) -> BlockPred {
        match &self {
            BlockPred::Cmp { property, .. } => {
                let property = *property;
                BlockPred::And(Box::new(self), Box::new(BlockPred::Cmp { property, op }))
            }
            _ => panic!("chained comparison requires a Cmp block predicate"),
        }
    }

    super::define_cmp_methods!(BlockPred, |self, op| self.chain_cmp(op));
}

impl ::std::ops::BitAnd for BlockPred {
    type Output = BlockPred;
    fn bitand(self, rhs: BlockPred) -> BlockPred {
        BlockPred::And(Box::new(self), Box::new(rhs))
    }
}

impl ::std::ops::BitOr for BlockPred {
    type Output = BlockPred;
    fn bitor(self, rhs: BlockPred) -> BlockPred {
        BlockPred::Or(Box::new(self), Box::new(rhs))
    }
}

impl ::std::ops::Not for BlockPred {
    type Output = BlockPred;
    fn not(self) -> BlockPred {
        BlockPred::Not(Box::new(self))
    }
}

// --- Shared result combinators ---

fn merge_results(l: Result<(), String>, r: Result<(), String>) -> Result<(), String> {
    match (l, r) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(e), Ok(())) | (Ok(()), Err(e)) => Err(e),
        (Err(e1), Err(e2)) => Err(format!("{}\n{}", e1, e2)),
    }
}

fn or_results(l: Result<(), String>, r: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    match l {
        Ok(()) => Ok(()),
        Err(e1) => match r() {
            Ok(()) => Ok(()),
            Err(e2) => Err(format!(
                "none of the alternatives succeeded:\n  {}\n  {}",
                e1.replace('\n', "\n  "),
                e2.replace('\n', "\n  ")
            )),
        },
    }
}

fn negate_result(r: Result<(), String>) -> Result<(), String> {
    if r.is_err() {
        Ok(())
    } else {
        Err("expected predicate to fail, but it succeeded".to_string())
    }
}

// --- Discrete predicate types ---

/// Single property comparison predicate.
pub struct Cmp {
    pub property: Property,
    pub op: CmpOp,
}

impl Predicate for Cmp {
    fn evaluate(
        &self,
        fn_name: &str,
        functions: &HashMap<String, FunctionIr>,
        _env: &EnvSpec,
    ) -> Result<(), String> {
        let ir = get_ir(fn_name, functions);
        self.op
            .check(ir.compute_property(self.property), self.property.name())
    }
}

/// Logical AND combinator.
pub struct And<L, R>(pub L, pub R);

impl<L: Predicate, R: Predicate> Predicate for And<L, R> {
    fn evaluate(
        &self,
        fn_name: &str,
        functions: &HashMap<String, FunctionIr>,
        env: &EnvSpec,
    ) -> Result<(), String> {
        merge_results(
            self.0.evaluate(fn_name, functions, env),
            self.1.evaluate(fn_name, functions, env),
        )
    }

    fn collect_environments(&self, envs: &mut Vec<EnvSpec>) {
        let mut lhs = Vec::new();
        let mut rhs = Vec::new();
        self.0.collect_environments(&mut lhs);
        self.1.collect_environments(&mut rhs);
        for l in &lhs {
            for r in &rhs {
                envs.push(l.and(r));
            }
        }
    }
}

/// Logical OR combinator.
pub struct Or<L, R>(pub L, pub R);

impl<L: Predicate, R: Predicate> Predicate for Or<L, R> {
    fn evaluate(
        &self,
        fn_name: &str,
        functions: &HashMap<String, FunctionIr>,
        env: &EnvSpec,
    ) -> Result<(), String> {
        or_results(self.0.evaluate(fn_name, functions, env), || {
            self.1.evaluate(fn_name, functions, env)
        })
    }

    fn collect_environments(&self, envs: &mut Vec<EnvSpec>) {
        let mut lhs = Vec::new();
        let mut rhs = Vec::new();
        self.0.collect_environments(&mut lhs);
        self.1.collect_environments(&mut rhs);

        fn insert_union(acc: &mut Vec<EnvSpec>, candidate: EnvSpec) {
            let mut pending = vec![candidate];
            for existing in acc.iter() {
                let mut next = Vec::new();
                for p in pending {
                    next.extend(p.or(existing));
                }
                pending = next;
            }
            for p in pending {
                if !acc.contains(&p) {
                    acc.push(p);
                }
            }
        }

        match (lhs.is_empty(), rhs.is_empty()) {
            (true, true) => {}
            (true, false) => envs.extend(rhs),
            (false, true) => envs.extend(lhs),
            (false, false) => {
                let mut merged = Vec::new();
                for l in lhs {
                    insert_union(&mut merged, l);
                }
                for r in rhs {
                    insert_union(&mut merged, r);
                }
                envs.extend(merged);
            }
        }
    }
}

/// Logical NOT combinator.
pub struct Not<P>(pub P);

impl<P: Predicate> Predicate for Not<P> {
    fn evaluate(
        &self,
        fn_name: &str,
        functions: &HashMap<String, FunctionIr>,
        env: &EnvSpec,
    ) -> Result<(), String> {
        negate_result(self.0.evaluate(fn_name, functions, env))
    }

    fn collect_environments(&self, envs: &mut Vec<EnvSpec>) {
        self.0.collect_environments(envs);
    }
}

/// Block quantifier predicate (all/any).
pub struct Quantifier {
    pub require_all: bool,
    pub f: Box<dyn Fn(super::dsl::BlockRef) -> BlockPred>,
}

impl Predicate for Quantifier {
    fn evaluate(
        &self,
        fn_name: &str,
        functions: &HashMap<String, FunctionIr>,
        _env: &EnvSpec,
    ) -> Result<(), String> {
        let ir = get_ir(fn_name, functions);
        for i in 0..ir.blocks.len() {
            let ok = (self.f)(super::dsl::BlockRef::new())
                .evaluate_block(ir, i)
                .is_ok();
            if self.require_all && !ok {
                return Err(format!("basic_blocks.all(): failed at block {}", i));
            }
            if !self.require_all && ok {
                return Ok(());
            }
        }
        if self.require_all {
            Ok(())
        } else {
            Err("basic_blocks.any(): no block satisfied the predicate".to_string())
        }
    }
}

/// Indexed block access predicate.
pub struct At {
    pub index: usize,
    pub pred: BlockPred,
}

impl Predicate for At {
    fn evaluate(
        &self,
        fn_name: &str,
        functions: &HashMap<String, FunctionIr>,
        _env: &EnvSpec,
    ) -> Result<(), String> {
        let ir = get_ir(fn_name, functions);
        self.pred.evaluate_block(ir, self.index).map_err(|e| {
            format!(
                "basic_blocks[{}]:\n  {}",
                self.index,
                e.replace('\n', "\n  ")
            )
        })
    }
}

// --- Display impls ---

impl fmt::Display for Cmp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.property.name(), self.op)
    }
}

impl<L: Predicate, R: Predicate> fmt::Display for And<L, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} && {})", self.0, self.1)
    }
}

impl<L: Predicate, R: Predicate> fmt::Display for Or<L, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} || {})", self.0, self.1)
    }
}

impl<P: Predicate> fmt::Display for Not<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "!({})", self.0)
    }
}

impl fmt::Display for Quantifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = if self.require_all { "all" } else { "any" };
        write!(f, "basic_blocks.{}(|bb| ...)", name)
    }
}

impl fmt::Display for At {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "basic_blocks.at({}).<prop>().len().<cmp>()", self.index)
    }
}

// --- Chained comparisons ---

impl Cmp {
    super::define_cmp_methods!(And<Cmp, Cmp>, |self, op| {
        let prop = self.property;
        And(self, Cmp { property: prop, op })
    });
}

impl<L: Predicate> And<L, Cmp> {
    super::define_cmp_methods!(And<And<L, Cmp>, Cmp>, |self, op| {
        let prop = self.1.property;
        And(self, Cmp { property: prop, op })
    });
}

impl At {
    fn extract_block_cmp_info(&self) -> (usize, Property) {
        if let BlockPred::Cmp { property, .. } = &self.pred {
            (self.index, *property)
        } else {
            panic!("chained comparison on At requires a Cmp block predicate")
        }
    }

    super::define_cmp_methods!(And<At, At>, |self, op| {
        let (index, property) = self.extract_block_cmp_info();
        And(self, At { index, pred: BlockPred::Cmp { property, op } })
    });
}

impl<L: Predicate> And<L, At> {
    super::define_cmp_methods!(And<And<L, At>, At>, |self, op| {
        let (index, property) = self.1.extract_block_cmp_info();
        And(
            self,
            At {
                index,
                pred: BlockPred::Cmp { property, op },
            },
        )
    });
}
// --- Operator impls ---

macro_rules! impl_predicate_ops {
    ($ty:ty) => {
        impl<Rhs: crate::Predicate> ::std::ops::BitAnd<Rhs> for $ty {
            type Output = crate::predicate::combinators::And<$ty, Rhs>;
            fn bitand(self, rhs: Rhs) -> Self::Output { crate::predicate::combinators::And(self, rhs) }
        }
        impl<Rhs: crate::Predicate> ::std::ops::BitOr<Rhs> for $ty {
            type Output = crate::predicate::combinators::Or<$ty, Rhs>;
            fn bitor(self, rhs: Rhs) -> Self::Output { crate::predicate::combinators::Or(self, rhs) }
        }
        impl ::std::ops::Not for $ty {
            type Output = crate::predicate::combinators::Not<$ty>;
            fn not(self) -> Self::Output { crate::predicate::combinators::Not(self) }
        }
    };
    ($name:ident [$($param:ident : $bound:path),+]) => {
        impl<$($param: $bound,)+ Rhs: crate::Predicate> ::std::ops::BitAnd<Rhs> for $name<$($param),+> {
            type Output = crate::predicate::combinators::And<$name<$($param),+>, Rhs>;
            fn bitand(self, rhs: Rhs) -> Self::Output { crate::predicate::combinators::And(self, rhs) }
        }
        impl<$($param: $bound,)+ Rhs: crate::Predicate> ::std::ops::BitOr<Rhs> for $name<$($param),+> {
            type Output = crate::predicate::combinators::Or<$name<$($param),+>, Rhs>;
            fn bitor(self, rhs: Rhs) -> Self::Output { crate::predicate::combinators::Or(self, rhs) }
        }
        impl<$($param: $bound),+> ::std::ops::Not for $name<$($param),+> {
            type Output = crate::predicate::combinators::Not<$name<$($param),+>>;
            fn not(self) -> Self::Output { crate::predicate::combinators::Not(self) }
        }
    };
}

impl_predicate_ops!(Cmp);
impl_predicate_ops!(Quantifier);
impl_predicate_ops!(At);
impl_predicate_ops!(And[L: Predicate, R: Predicate]);
impl_predicate_ops!(Or[L: Predicate, R: Predicate]);
impl_predicate_ops!(Not[P: Predicate]);
