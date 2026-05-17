use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::env::{EnvSpec, OptLevel};
use crate::ir::{parse_ir_functions, FunctionIr};

/// Compute a unique target-dir for a given environment.
///
/// Target triples are already isolated by cargo into `<triple>/release/deps/`
/// subdirectories, so only `rustc` and `opt_level` need additional separation.
///
/// Examples (given base `/tmp/ir-assert-target`):
///   - default env          → `/tmp/ir-assert-target`
///   - opt_level("2")       → `/tmp/ir-assert-target-opt2`
///   - rustc("1.86")        → `/tmp/ir-assert-target-rustc_1.86`
///   - rustc("1.86")+opt(2) → `/tmp/ir-assert-target-rustc_1.86-opt2`
fn env_target_dir(base: &Path, env: &EnvSpec) -> PathBuf {
    let mut suffix = String::new();
    if let Some(version) = env.rustc {
        suffix.push_str(&format!("-rustc_{}", version));
    }
    match &env.opt_level {
        OptLevel::None | OptLevel::Release => {}
        OptLevel::Debug => suffix.push_str("-debug"),
        OptLevel::OptLevel(level) => suffix.push_str(&format!("-opt{}", level)),
    }
    if suffix.is_empty() {
        base.to_path_buf()
    } else {
        let mut s = base.as_os_str().to_os_string();
        s.push(&suffix);
        PathBuf::from(s)
    }
}

/// Ensures only one thread builds the IR file at a time.
pub(crate) static BUILD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Check if a rustc toolchain is available via cargo.
/// Uses "cargo" from PATH (the rustup proxy) since `+version` syntax
/// is handled by the proxy, not the toolchain-specific binary.
fn check_toolchain_available(rustup: &str, version: &str) -> bool {
    Command::new(rustup)
        .args(["run", &format!("{}", version), "cargo"])
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Build IR for a specific environment using `cargo rustc`.
pub(crate) fn build_ir_for_env(
    cargo: &str,
    rustup: &str,
    manifest_dir: &str,
    crate_name: &str,
    is_test: bool,
    env: &EnvSpec,
    ir_target_dir: &Path,
) -> Result<(), String> {
    if let Some(version) = env.rustc {
        if !check_toolchain_available(rustup, version) {
            return Err(format!("toolchain {} is not available", version));
        }
    }

    // Use "cargo" from PATH (rustup proxy) for non-default toolchains
    // since `+version` syntax requires the proxy, not the toolchain binary.
    // Use `$CARGO` for the default toolchain for exact compatibility.
    let mut cmd = if env.rustc.is_some() {
        let mut c = Command::new(rustup);
        c.args(["run", &format!("{}", env.rustc.unwrap()), "cargo"]);
        c
    } else {
        Command::new(cargo)
    };
    cmd.arg("rustc");
    cmd.args(["--manifest-path", &format!("{}/Cargo.toml", manifest_dir)]);
    if is_test {
        cmd.args(["--test", crate_name]);
    } else {
        cmd.arg("--lib");
    }
    let is_debug = matches!(env.opt_level, OptLevel::Debug);
    if !is_debug {
        cmd.arg("--release");
    }
    if let Some(target) = env.target {
        cmd.args(["--target", target]);
    }
    cmd.args(["--target-dir", ir_target_dir.to_str().unwrap()]);
    if let OptLevel::OptLevel(level) = &env.opt_level {
        cmd.args(["--config", &format!("profile.release.opt-level={}", level)]);
    }
    cmd.arg("--");
    cmd.arg("--emit=llvm-ir");
    cmd.args(["-C", "strip=debuginfo", "-C", "codegen-units=1"]);

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let output = cmd
        .output()
        .map_err(|e| format!("cannot execute cargo: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "build for {} failed with status: {}\n{}",
            env, output.status, stderr
        ));
    }

    Ok(())
}

/// Find the .ll file produced by `cargo rustc`.
/// Looks in `<ir_target_dir>/[<triple>/]release/deps/` for `<crate_name>-*.ll`,
/// returning the most recently modified match.
fn find_ll_file(
    ir_target_dir: &Path,
    crate_name: &str,
    target: Option<&str>,
    is_debug: bool,
) -> Option<PathBuf> {
    let profile_dir = if is_debug { "debug" } else { "release" };
    let deps_dir = if let Some(triple) = target {
        ir_target_dir.join(triple).join(profile_dir).join("deps")
    } else {
        ir_target_dir.join(profile_dir).join("deps")
    };

    // TODO: use exe name instead of crate name to make prefix.
    let prefix = format!("{}-", crate_name.replace('-', "_"));

    let entries = match std::fs::read_dir(&deps_dir) {
        Ok(entries) => entries,
        Err(_) => return None,
    };

    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("ll") {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(&prefix) {
                    let mtime = entry
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .unwrap_or(std::time::UNIX_EPOCH);
                    if best.as_ref().map_or(true, |(_, t)| mtime > *t) {
                        best = Some((path, mtime));
                    }
                }
            }
        }
    }

    best.map(|(p, _)| p)
}

/// Load and parse an IR file. Returns parsed functions.
pub(crate) fn load_ir(path: &Path, allow_debug: bool) -> Option<Vec<FunctionIr>> {
    if !path.exists() {
        return None;
    }
    let content =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("Cannot read file: {}", e));

    // Check for debug mode
    if !allow_debug && content.contains("__rustc_debug_gdb_scripts_section__") {
        panic!("The ir is emitted within debug mode.");
    }

    let functions = parse_ir_functions(&content);
    Some(functions)
}

/// Find function references within a container function's IR.
/// The container uses inline asm with `ptrtoint (ptr @symbol to i64)` patterns
/// to reference the target functions. We extract all `@symbol` references from
/// the container's IR body, excluding the function definition line itself.
pub(crate) fn find_referenced_functions(container: &FunctionIr) -> Vec<String> {
    let mut refs = Vec::new();
    let mut first_line = true;
    for line in container.raw.lines() {
        // Skip the `define ... @container_name(...) {` line
        if first_line {
            first_line = false;
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') {
            continue;
        }
        // Skip the closing brace
        if trimmed == "}" {
            continue;
        }
        // Look for all @symbol references in any instruction
        for (i, _) in trimmed.match_indices('@') {
            let after_at = &trimmed[i + 1..];
            // Handle quoted names: @"..."
            let name = if after_at.starts_with('"') {
                if let Some(end) = after_at[1..].find('"') {
                    &after_at[1..end + 1]
                } else {
                    continue;
                }
            } else {
                // Unquoted: ends at ( or space or , or ) or end
                let end = after_at
                    .find(|c: char| c == '(' || c == ' ' || c == ',' || c == ')' || c == '\n')
                    .unwrap_or(after_at.len());
                &after_at[..end]
            };
            if !name.is_empty()
                && !name.starts_with("llvm.")
                && !name.starts_with("ir_assert_container_")
            {
                refs.push(name.to_string());
            }
        }
    }
    refs
}

/// Load IR for all environments and build a mapping from EnvSpec to functions list.
/// Returns Ok with the mapping if at least one environment succeeded,
/// or Err with all collected errors if every environment failed.
pub(crate) fn load_all_envs(
    cargo: &str,
    rustup: &str,
    manifest_dir: &str,
    crate_name: &str,
    is_test: bool,
    envs: &[EnvSpec],
    ir_target_dir: &Path,
) -> Result<HashMap<EnvSpec, Vec<FunctionIr>>, HashMap<EnvSpec, String>> {
    let mut result = HashMap::new();
    let mut errors = HashMap::new();

    for env in envs {
        let target_dir = env_target_dir(ir_target_dir, env);
        match build_ir_for_env(
            cargo,
            rustup,
            manifest_dir,
            crate_name,
            is_test,
            env,
            &target_dir,
        ) {
            Ok(()) => {
                let is_debug = matches!(env.opt_level, OptLevel::Debug);
                if let Some(ll_path) = find_ll_file(&target_dir, crate_name, env.target, is_debug)
                {
                    if let Some(funcs) = load_ir(&ll_path, is_debug) {
                        result.insert(env.clone(), funcs);
                        continue;
                    }
                }
                errors.insert(env.clone(), "IR file not found after build".to_string());
            }
            Err(e) => {
                errors.insert(env.clone(), e);
            }
        }
    }

    if !errors.is_empty() {
        Err(errors)
    } else {
        Ok(result)
    }
}
