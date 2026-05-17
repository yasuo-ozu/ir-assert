use ir_assert::assert_ir;
use std::process::Command;
use std::process::Stdio;

fn simple_add(a: u64, b: u64) -> u64 {
    a + b
}

fn identity(x: u64) -> u64 {
    x
}

/// Test env Or with an unavailable target — strict env availability should fail.
#[test]
#[should_panic(expected = "none of the environments are available")]
fn test_env_or_with_unavailable() {
    assert_ir!(
        (target("wasm32-unknown-unknown") | target("x86_64-unknown-linux-gnu"))
            & basic_blocks.len().eq(1),
        simple_add
    );
}

/// Test rustc(X) && rustc(Y) with two different versions.
/// And of env predicates with different versions always fails because
/// no single environment can match both simultaneously.
#[test]
#[should_panic(expected = "EnvSpec::and conflict on field `rustc`")]
fn test_rustc_and_different_versions() {
    assert_ir!(
        rustc("1.80") & rustc("1.90") & basic_blocks.len().eq(1),
        simple_add
    );
}

/// Test rustc(X) || rustc(Y) with two different installed toolchains.
/// 1.80 is ABI-incompatible but 1.90 matches the default, so Or succeeds
/// because at least one env satisfies the predicate.
#[test]
fn test_rustc_or_different_versions() {
    assert_ir!(
        (rustc("1.80") | rustc("1.90")) & basic_blocks.len().eq(1),
        simple_add
    );
}

/// Test rustc(X) && rustc(Y) where both are unavailable — should fail.
/// And of different env predicates always fails since no single env matches both.
#[test]
#[should_panic(expected = "EnvSpec::and conflict on field `rustc`")]
fn test_rustc_and_both_unavailable() {
    assert_ir!(
        rustc("99.98") & rustc("99.99") & basic_blocks.len().eq(1),
        simple_add
    );
}

/// Test rustc(X) || rustc(Y) where X is unavailable but Y exists.
/// Strict env availability requires all collected environments to be buildable.
#[test]
#[should_panic(expected = "none of the environments are available")]
fn test_rustc_or_one_unavailable() {
    assert_ir!(
        (rustc("99.99") | rustc("1.90")) & basic_blocks.len().eq(1),
        simple_add
    );
}

/// Test rustc(X) || rustc(Y) where both are unavailable — should fail.
#[test]
#[should_panic(expected = "none of the environments")]
fn test_rustc_or_both_unavailable() {
    assert_ir!(
        (rustc("99.98") | rustc("99.99")) & basic_blocks.len().eq(1),
        simple_add
    );
}

/// Test rustc(X) && rustc(Y) where both are available but have different version strings.
/// And of env predicates with different version strings always fails because
/// no single environment can match both simultaneously.
#[test]
#[should_panic(expected = "EnvSpec::and conflict on field `rustc`")]
fn test_rustc_and_both_available() {
    assert_ir!(
        rustc("1.90") & rustc("1.90.0") & basic_blocks.len().eq(1),
        simple_add
    );
}

/// Test rustc(X) || rustc(Y) where both are available and compilable.
#[test]
fn test_rustc_or_both_available() {
    assert_ir!(
        (rustc("1.90") | rustc("1.90.0")) & basic_blocks.len().eq(1),
        simple_add
    );
}

/// Test target(host) && target(host) — same target twice merges without conflict.
#[test]
fn test_target_and_host_twice() {
    assert_ir!(
        target("x86_64-unknown-linux-gnu")
            & target("x86_64-unknown-linux-gnu")
            & basic_blocks.len().eq(1),
        simple_add
    );
}

/// Test target(X) || target(Y) where X is unavailable but Y (host) works.
/// Strict env availability requires all collected environments to be buildable.
#[test]
#[should_panic(expected = "none of the environments are available")]
fn test_target_or_one_unavailable() {
    assert_ir!(
        (target("wasm32-unknown-unknown") | target("x86_64-unknown-linux-gnu"))
            & basic_blocks.len().eq(1),
        simple_add
    );
}

/// Test target(X) && target(Y) where one target is unavailable — should fail.
/// And of different target env predicates always fails since no single env matches both.
#[test]
#[should_panic(expected = "EnvSpec::and conflict on field `target`")]
fn test_target_and_one_unavailable() {
    assert_ir!(
        target("x86_64-unknown-linux-gnu")
            & target("wasm32-unknown-unknown")
            & basic_blocks.len().eq(1),
        simple_add
    );
}

/// Test target(X) || target(Y) where both are unavailable — should fail.
#[test]
#[should_panic(expected = "none of the environments")]
fn test_target_or_both_unavailable() {
    assert_ir!(
        (target("nonearch-unknown-linux-gnu") | target("nonearch-unknown-unknown"))
            & basic_blocks.len().eq(1),
        simple_add
    );
}

/// Test target(X) && target(Y) where both are unavailable — should fail.
/// And of different target env predicates always fails since no single env matches both.
#[test]
#[should_panic]
fn test_target_and_both_unavailable() {
    assert_ir!(
        (target("nonearch-unknown-linux-gnu") | target("nonearch-unknown-unknown"))
            & rustc("1.80")
            & basic_blocks.len().eq(1),
        simple_add
    );
}

#[test]
fn test_opt_level_predicate() {
    assert_ir!(opt0 & basic_blocks.len().eq(1), identity);
}

#[test]
#[should_panic(expected = "none of the environments are available")]
fn test_tier0_target_helper() {
    assert_ir!(
        (target_wasm32_unknown_unknown | target_x86_64_unknown_linux_gnu)
            & basic_blocks.len().eq(1),
        identity
    );
}

fn sum3_wrapping(a: u64, b: u64, c: u64) -> u64 {
    a.wrapping_add(b).wrapping_add(c)
}

#[test]
fn test_opt0_and_opt3_exact_counts_differ_on_specific_target() {
    assert_ir!(
        (target_x86_64_unknown_linux_gnu & opt0 & instructions.len().ge(3))
            | (target_x86_64_unknown_linux_gnu & opt3 & instructions.len().le(3)),
        sum3_wrapping
    );
}

#[test]
fn test_rustup_run_versions_and_targets() {
    let version_targets = vec![
        (None, Some("wasm32-unknown-unknown")),
        (None, Some("x86_64-unknown-linux-gnu")),
        (Some("1.80"), None),
        (Some("1.90"), None),
        (Some("1.80"), Some("wasm32-unknown-unknown")),
        (Some("1.80"), Some("x86_64-unknown-linux-gnu")),
    ];

    let mut err_count = 0;
    for (version, target) in version_targets {
        let mut cmd = if let Some(version) = version {
            let mut cmd = Command::new("rustup");
            cmd.args(["run", version, "cargo"]);
            cmd
        } else {
            Command::new("cargo")
        };
        cmd.arg("rustc");
        if let Some(target) = target {
            cmd.args(["--target", target]);
        }
        let status = cmd
            .args(["--", "--version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap_or_else(|e| panic!("failed to execute rustup or cargo: {e}"));
        if !status.success() {
            eprintln!("cannot find toolchain {version:?} and target {target:?}");
            err_count += 1;
        }
    }
    assert_eq!(
        err_count, 0,
        "please install correct toolchain or target to run tests"
    );
}
