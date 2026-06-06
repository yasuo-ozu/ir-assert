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

/// Cache key: (manifest_dir, crate_name, is_test, env).
type CacheKey = (String, String, bool, EnvSpec);

type BuildCache = std::sync::Mutex<HashMap<CacheKey, Result<Vec<FunctionIr>, String>>>;

/// Process-global IR cache.
///
/// The Mutex serialises concurrent builds (only one thread compiles at a time)
/// and protects the cache.  When the result for an env is already present, the
/// compiler is not re-invoked — even when a second `assert_ir!` call in the same
/// test binary requests the same (crate, env) combination.
pub(crate) static BUILD_LOCK: std::sync::OnceLock<BuildCache> = std::sync::OnceLock::new();

/// Shared build parameters that are constant for one `assert_ir!` invocation.
pub(crate) struct BuildContext<'a> {
    pub cargo: &'a str,
    pub rustup: &'a str,
    pub manifest_dir: &'a str,
    pub crate_name: &'a str,
    pub is_test: bool,
}

/// Check if a rustc toolchain is available via cargo.
/// Uses "cargo" from PATH (the rustup proxy) since `+version` syntax
/// is handled by the proxy, not the toolchain-specific binary.
fn check_toolchain_available(rustup: &str, version: &str) -> bool {
    Command::new(rustup)
        .args(["run", version, "cargo"])
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Build IR for a specific environment using `cargo rustc`.
pub(crate) fn build_ir_for_env(
    ctx: &BuildContext<'_>,
    env: &EnvSpec,
    ir_target_dir: &Path,
) -> Result<(), String> {
    if let Some(version) = env.rustc {
        if !check_toolchain_available(ctx.rustup, version) {
            return Err(format!("toolchain {} is not available", version));
        }
    }

    // Use "cargo" from PATH (rustup proxy) for non-default toolchains
    // since `+version` syntax requires the proxy, not the toolchain binary.
    // Use `$CARGO` for the default toolchain for exact compatibility.
    let mut cmd = if let Some(rustc) = &env.rustc {
        let mut c = Command::new(ctx.rustup);
        c.args(["run", rustc, "cargo"]);
        c
    } else {
        Command::new(ctx.cargo)
    };
    cmd.arg("rustc");
    cmd.args(["--manifest-path", &format!("{}/Cargo.toml", ctx.manifest_dir)]);
    if ctx.is_test {
        cmd.args(["--test", ctx.crate_name]);
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

    // Use a no-op linker: we only need the .ll file, not a linked binary.
    // cargo always adds --emit=link which triggers linking; suppress it by
    // replacing the linker with a command that immediately returns success.
    if cfg!(windows) {
        let noop_linker = ir_target_dir.join("noop_linker.bat");
        if !noop_linker.exists() {
            let _ = std::fs::create_dir_all(ir_target_dir);
            let _ = std::fs::write(&noop_linker, "@exit /b 0\r\n");
        }
        cmd.arg("-C");
        cmd.arg(format!("linker={}", noop_linker.display()));
    } else {
        cmd.args(["-C", "linker=true"]);
    }

    // Signal to the proc-macro that this is an internal IR-generation invocation so that
    // debug_assert_ir! with multiple targets does not abort during this sub-compilation.
    cmd.env("IR_ASSERT_IR_GEN", "1");

    // In wasm32-unknown-unknown, inline assembly is unstable
    cmd.env("RUSTC_BOOTSTRAP", "1");
    cmd.arg("-Zcrate-attr=feature(asm_experimental_arch)");

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

    let mut best_prefix_match: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut best_any_match: Option<(PathBuf, std::time::SystemTime)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("ll") {
            let mtime = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(std::time::UNIX_EPOCH);
            if best_any_match.as_ref().map_or(true, |(_, t)| mtime > *t) {
                best_any_match = Some((path.clone(), mtime));
            }
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(&prefix)
                    && best_prefix_match.as_ref().map_or(true, |(_, t)| mtime > *t)
                {
                    best_prefix_match = Some((path, mtime));
                }
            }
        }
    }

    best_prefix_match.or(best_any_match).map(|(path, _)| path)
}

/// Load and parse an IR file. Returns parsed functions.
pub(crate) fn load_ir(path: &Path, _allow_debug: bool) -> Option<Vec<FunctionIr>> {
    if !path.exists() {
        return None;
    }
    let content =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("Cannot read file: {}", e));

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
            let name = if let Some(stripped) = after_at.strip_prefix('"') {
                if let Some(end) = stripped.find('"') {
                    &stripped[..end]
                } else {
                    continue;
                }
            } else {
                // Unquoted: ends at ( or space or , or ) or end
                let end = after_at
                    .find(['(', ' ', ',', ')', '\n'])
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

/// Load IR for all environments, using `cache` to skip already-built envs.
///
/// Returns `Ok` with the per-env function map when every requested environment
/// built successfully, or `Err` with per-env error strings otherwise.
pub(crate) fn load_all_envs(
    ctx: &BuildContext<'_>,
    envs: &[EnvSpec],
    ir_target_dir: &Path,
    cache: &mut HashMap<CacheKey, Result<Vec<FunctionIr>, String>>,
) -> Result<HashMap<EnvSpec, Vec<FunctionIr>>, HashMap<EnvSpec, String>> {
    let mut result = HashMap::new();
    let mut errors = HashMap::new();

    for env in envs {
        let key: CacheKey = (
            ctx.manifest_dir.to_owned(),
            ctx.crate_name.to_owned(),
            ctx.is_test,
            env.clone(),
        );

        // Build only if not already cached (hit or error).
        if !cache.contains_key(&key) {
            let target_dir = env_target_dir(ir_target_dir, env);
            let entry = build_ir_for_env(ctx, env, &target_dir)
            .and_then(|()| {
                let is_debug = matches!(env.opt_level, OptLevel::Debug);
                find_ll_file(&target_dir, ctx.crate_name, env.target, is_debug)
                    .and_then(|ll_path| load_ir(&ll_path, is_debug))
                    .ok_or_else(|| "IR file not found after build".to_string())
            });
            cache.insert(key.clone(), entry);
        }

        match cache[&key].clone() {
            Ok(funcs) => {
                result.insert(env.clone(), funcs);
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
