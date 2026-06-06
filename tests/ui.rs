#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}

#[test]
fn ui_no_debug_assertions() {
    // Compile with debug_assertions disabled to exercise the compile_error! path in
    // debug_assert_ir! when multiple targets are given.
    let prev = std::env::var("RUSTFLAGS").ok();
    let flags = format!(
        "{} -C debug-assertions=no",
        prev.as_deref().unwrap_or("")
    )
    .trim()
    .to_string();
    std::env::set_var("RUSTFLAGS", &flags);

    // Scope ensures TestCases drops (and runs tests) before RUSTFLAGS is restored.
    {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/ui_no_debug/*.rs");
    }

    match prev {
        Some(v) => std::env::set_var("RUSTFLAGS", v),
        None => std::env::remove_var("RUSTFLAGS"),
    }
}
