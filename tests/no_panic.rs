use ir_assert::assert_ir;

fn simple_add(a: u64, b: u64) -> u64 {
    a.wrapping_add(b)
}

fn identity(x: u64) -> u64 {
    x
}

fn bitwise_xor(a: u32, b: u32) -> u32 {
    a ^ b
}

fn multiply(a: i64, b: i64) -> i64 {
    a.wrapping_mul(b)
}

fn max_wrapping(a: i64, b: i64) -> i64 {
    if a > b {
        a
    } else {
        b
    }
}

fn saturating_add_std(a: u64, b: u64) -> u64 {
    a.saturating_add(b)
}

fn checked_mul_default(a: u64, b: u64) -> u64 {
    a.checked_mul(b).unwrap_or(0)
}

fn map_or_default_len(s: Option<&str>) -> usize {
    s.map(str::len).unwrap_or(0)
}

fn clamp_std(x: i32) -> i32 {
    x.clamp(-10, 10)
}

fn rotate_and_count_ones(x: u32) -> u32 {
    x.rotate_left(5).count_ones()
}

fn safe_slice_get(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

fn string_starts_with_ascii(s: &str) -> bool {
    s.starts_with('a')
}

fn slice_index(s: &[u8], i: usize) -> u8 {
    s[i]
}

fn unwrap_option(x: Option<u64>) -> u64 {
    x.unwrap()
}

fn expect_option(x: Option<u64>) -> u64 {
    x.expect("missing value")
}

fn checked_add_or_panic(a: u64, b: u64) -> u64 {
    a.checked_add(b).unwrap()
}

fn checked_add_or_expect(a: u64, b: u64) -> u64 {
    a.checked_add(b).expect("overflow")
}

fn divide(a: u64, b: u64) -> u64 {
    a / b
}

fn explicit_panic(x: u64) -> u64 {
    if x == 0 {
        panic!("zero!");
    }
    x
}

fn remainder(a: u64, b: u64) -> u64 {
    a % b
}

fn assert_nonzero(x: u64) -> u64 {
    assert!(x != 0, "zero!");
    x
}

fn debug_assert_nonzero(x: u64) -> u64 {
    debug_assert!(x != 0, "zero!");
    x
}

fn vec_remove_index(v: &mut Vec<u8>, i: usize) -> u8 {
    v.remove(i)
}

fn str_slice_range(s: &str, n: usize) -> &str {
    &s[..n]
}

fn repeat_nth_char(s: &str, i: usize) -> char {
    s.chars().nth(i).unwrap()
}

fn btree_get_or_panic(
    m: &std::collections::BTreeMap<u32, u32>,
    k: u32,
) -> u32 {
    *m.get(&k).unwrap()
}

#[test]
fn test_no_panic_wrapping_add() {
    assert_ir!(no_panic, simple_add);
}

#[test]
fn test_no_panic_identity() {
    assert_ir!(no_panic, identity);
}

#[test]
fn test_no_panic_closure() {
    assert_ir!(no_panic, |a: u64, b: u64| a.wrapping_add(b));
}

#[test]
fn test_no_panic_combined() {
    assert_ir!(no_panic & calls.len().eq(0), simple_add);
}

#[test]
fn test_no_panic_bitwise() {
    assert_ir!(no_panic, bitwise_xor);
}

#[test]
fn test_no_panic_multiply() {
    assert_ir!(no_panic, multiply);
}

#[test]
fn test_no_panic_max() {
    assert_ir!(no_panic, max_wrapping);
}

#[test]
fn test_no_panic_multiple_targets() {
    assert_ir!(no_panic, simple_add, identity, bitwise_xor);
}

#[test]
fn test_no_panic_saturating_add_std() {
    assert_ir!(no_panic, saturating_add_std);
}

#[test]
fn test_no_panic_checked_mul_default() {
    assert_ir!(no_panic, checked_mul_default);
}

#[test]
fn test_no_panic_map_or_default_len() {
    assert_ir!(no_panic, map_or_default_len);
}

#[test]
fn test_no_panic_clamp_std() {
    assert_ir!(no_panic, clamp_std);
}

#[test]
fn test_no_panic_rotate_and_count_ones() {
    assert_ir!(no_panic, rotate_and_count_ones);
}

#[test]
fn test_no_panic_safe_slice_get() {
    assert_ir!(no_panic, safe_slice_get);
}

#[test]
fn test_no_panic_string_starts_with_ascii() {
    assert_ir!(no_panic, string_starts_with_ascii);
}

#[test]
fn test_not_no_panic_slice_index() {
    assert_ir!(!no_panic, slice_index);
}

#[test]
fn test_not_no_panic_unwrap() {
    assert_ir!(!no_panic, unwrap_option);
}

#[test]
fn test_not_no_panic_expect() {
    assert_ir!(!no_panic, expect_option);
}

#[test]
fn test_not_no_panic_checked_add_unwrap() {
    assert_ir!(!no_panic, checked_add_or_panic);
}

#[test]
fn test_not_no_panic_checked_add_expect() {
    assert_ir!(!no_panic, checked_add_or_expect);
}

#[test]
fn test_not_no_panic_divide() {
    // Division by zero panics
    assert_ir!(!no_panic, divide);
}

#[test]
fn test_not_no_panic_explicit_panic() {
    assert_ir!(!no_panic, explicit_panic);
}

#[test]
fn test_not_no_panic_assert() {
    assert_ir!(!no_panic, assert_nonzero);
}

#[test]
fn test_no_panic_debug_assert() {
    assert_ir!(no_panic, debug_assert_nonzero);
}

#[test]
fn test_not_no_panic_vec_remove_index() {
    assert_ir!(!no_panic, vec_remove_index);
}

#[test]
fn test_not_no_panic_str_slice_range() {
    assert_ir!(!no_panic, str_slice_range);
}

#[test]
fn test_not_no_panic_repeat_nth_char() {
    assert_ir!(!no_panic, repeat_nth_char);
}

#[test]
fn test_not_no_panic_btree_get_or_panic() {
    assert_ir!(!no_panic, btree_get_or_panic);
}

#[test]
fn test_not_no_panic_remainder() {
    // Remainder by zero panics
    assert_ir!(!no_panic, remainder);
}

#[test]
fn test_not_no_panic_closure_with_index() {
    assert_ir!(!no_panic, |s: &[u8], i: usize| s[i]);
}

#[test]
fn test_no_panic_and_single_block() {
    assert_ir!(no_panic & basic_blocks.len().eq(1), identity);
}

#[test]
fn test_not_no_panic_or_no_calls() {
    // unwrap_option panics, so !no_panic holds; combined with || this passes
    assert_ir!(!no_panic | calls.len().eq(0), unwrap_option);
}

// --- debug predicate tests ---

#[test]
fn test_debug_build_identity() {
    // identity is trivial even in debug mode
    assert_ir!(debug & basic_blocks.len().eq(1), identity);
}

#[test]
fn test_debug_build_debug_assert_has_calls() {
    // In debug mode, debug_assert! is present and generates call instructions
    assert_ir!(debug & calls.len().gt(0), debug_assert_nonzero);
}

#[test]
fn test_debug_build_simple_add() {
    // wrapping_add compiles to a single basic block even in debug mode
    assert_ir!(debug & calls.len().eq(0), simple_add);
}

#[test]
#[should_panic(expected = "EnvSpec::and conflict on field `opt_level`")]
fn test_debug_and_no_panic_must_fail() {
    // debug (OptLevel::Debug) conflicts with no_panic (OptLevel::Release)
    assert_ir!(debug & no_panic, debug_assert_nonzero);
}

#[test]
#[should_panic(expected = "EnvSpec::and conflict on field `opt_level`")]
fn test_debug_and_opt3_must_fail() {
    // debug (OptLevel::Debug) conflicts with opt3 (OptLevel::OptLevel("3"))
    assert_ir!(debug & opt3 & basic_blocks.len().eq(1), identity);
}

#[test]
#[should_panic(expected = "EnvSpec::and conflict on field `opt_level`")]
fn test_opt0_and_opt3_must_fail() {
    // OptLevel::OptLevel("0") conflicts with OptLevel::OptLevel("3")
    assert_ir!(opt0 & opt3 & basic_blocks.len().eq(1), identity);
}
