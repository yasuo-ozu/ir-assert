use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Span, TokenStream};
use proc_macro_error2::{abort, abort_call_site, proc_macro_error};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::*;
use template_quote::{quote, ToTokens};

/// Generate a unique hash from the macro input tokens and the span of the predicate.
///
/// We use the predicate's token spans (source-file locations) rather than a global counter
/// so the hash is identical in debug and release compilations of the same file.
/// A counter would desync when `#[cfg]` attributes skip some macro call sites in release
/// builds (e.g. a test function gated on `cfg(debug_assertions)`).
fn unique_hash(input: &TokenStream, predicate: &Expr) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    let s = input.to_string();
    let normalized: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    normalized.hash(&mut hasher);
    // The predicate tokens carry real source spans even inside macro expansions.
    let span_dbg: String = predicate
        .to_token_stream()
        .into_iter()
        .map(|t| format!("{:?}", t.span()))
        .collect::<Vec<_>>()
        .join(",");
    span_dbg.hash(&mut hasher);
    hasher.finish()
}

/// Classify a target expression for codegen.
enum Target<'a> {
    Closure {
        coerce_ident: Ident,
        arity: usize,
        params: &'a Punctuated<Pat, Token![,]>,
        body: &'a Expr,
    },
    Function(&'a Expr),
}

/// Parsed and prepared macro inputs used by both assert_ir! and debug_assert_ir! codegen.
struct CodegenInput<'a> {
    krate: TokenStream,
    container_ident: Ident,
    container_name: String,
    pred_tokens: TokenStream,
    pred_str: LitStr,
    target_str_lits: Vec<LitStr>,
    cargo_path: String,
    rustup_path: String,
    manifest_dir: String,
    crate_name: String,
    is_test: bool,
    asm_tag: LitStr,
    prepared: Vec<Target<'a>>,
}

impl<'a> CodegenInput<'a> {
    fn parse(crate_expr: &Expr, predicate_expr: &'a Expr, targets: &[&'a Expr]) -> Self {
        let krate: TokenStream = quote! { #crate_expr };

        let hash_input: TokenStream = {
            let pred_ts: TokenStream = quote! { #predicate_expr };
            let targets_ts: Vec<TokenStream> = targets.iter().map(|t| quote! { #t }).collect();
            quote! { #pred_ts #(#targets_ts)* }
        };
        let r = unique_hash(&hash_input, predicate_expr);

        let container_name = format!("ir_assert_container_{}", r);
        let container_ident = Ident::new(&container_name, Span::call_site());

        let pred_str = LitStr::new(
            &predicate_expr.to_token_stream().to_string(),
            Span::call_site(),
        );
        let target_str_lits: Vec<LitStr> = targets
            .iter()
            .map(|t| LitStr::new(&quote!(#t).to_string(), Span::call_site()))
            .collect();

        let pred_tokens = quote! { #predicate_expr };

        let cargo_path = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let rustup_path = std::env::var("RUSTUP").unwrap_or_else(|_| "rustup".to_string());
        let manifest_dir =
            std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");

        let args: Vec<String> = std::env::args().collect();
        let is_test = args.iter().any(|a| a == "--test");
        let crate_name = args
            .iter()
            .position(|a| a == "--crate-name")
            .and_then(|i| args.get(i + 1))
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        let asm_tag = LitStr::new(&format!("/* ir_assert {} {{0}} */", r), Span::call_site());

        let prepared: Vec<Target<'a>> = targets
            .iter()
            .enumerate()
            .map(|(i, target)| {
                if let Expr::Closure(closure) = target {
                    Target::Closure {
                        coerce_ident: Ident::new(
                            &format!("__ir_assert_fn_{}", i),
                            Span::call_site(),
                        ),
                        arity: closure.inputs.len(),
                        params: &closure.inputs,
                        body: &closure.body,
                    }
                } else {
                    Target::Function(target)
                }
            })
            .collect();

        Self {
            krate,
            container_ident,
            container_name,
            pred_tokens,
            pred_str,
            target_str_lits,
            cargo_path,
            rustup_path,
            manifest_dir,
            crate_name,
            is_test,
            asm_tag,
            prepared,
        }
    }

    /// Inline-asm statements that pin each target symbol inside the container function.
    fn target_stmts(&self) -> Vec<TokenStream> {
        let asm_tag = &self.asm_tag;
        self.prepared
            .iter()
            .map(|t| match t {
                Target::Closure {
                    coerce_ident,
                    params,
                    body,
                    ..
                } => {
                    let container_arg_tys: Vec<TokenStream> = params
                        .iter()
                        .map(|p| {
                            if matches!(p, Pat::Type(_)) {
                                quote! { _ }
                            } else {
                                quote! { usize }
                            }
                        })
                        .collect();
                    let container_params: Vec<TokenStream> = params
                        .iter()
                        .map(|p| {
                            if matches!(p, Pat::Type(_)) {
                                quote! { #p }
                            } else {
                                quote! { #p: usize }
                            }
                        })
                        .collect();
                    quote! {
                        let #coerce_ident: fn(#(#container_arg_tys),*) -> _ = |#(#container_params),*| #body;

                        #[cfg(target_arch = "wasm32")]
                        unsafe {
                            core::arch::asm!(#asm_tag, in(local) #coerce_ident as usize,
                                options(nostack, preserves_flags, readonly));
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        unsafe {
                            core::arch::asm!(#asm_tag, in(reg) #coerce_ident as usize,
                                options(nostack, preserves_flags, readonly));
                        }
                    }
                }
                Target::Function(expr) => quote! {
                    #[cfg(target_arch = "wasm32")]
                    unsafe {
                        core::arch::asm!(#asm_tag, in(local) #expr as usize,
                            options(nostack, preserves_flags, readonly));
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    unsafe {
                        core::arch::asm!(#asm_tag, in(reg) #expr as usize,
                            options(nostack, preserves_flags, readonly));
                    }
                },
            })
            .collect()
    }

    /// The `#[no_mangle]` container function that embeds target symbol references via asm.
    ///
    /// Always compiled (including in release) so the IR-generation pass can locate the
    /// container and discover the referenced target symbols.
    fn container_fn(&self) -> TokenStream {
        let target_stmts = self.target_stmts();
        let container_ident = &self.container_ident;
        quote! {
            #[no_mangle]
            #[inline(never)]
            #[allow(unused, dead_code)]
            fn #container_ident() {
                #(#target_stmts)*
            }
        }
    }

    /// The `__macro_internal(...)` call that drives the actual IR assertion at runtime.
    fn macro_internal_call(&self) -> TokenStream {
        let Self {
            krate,
            container_name,
            pred_tokens,
            pred_str,
            target_str_lits,
            cargo_path,
            rustup_path,
            manifest_dir,
            crate_name,
            is_test,
            ..
        } = self;
        quote! {
            #krate::__macro_internal(
                #cargo_path,
                #rustup_path,
                #manifest_dir,
                #crate_name,
                #is_test,
                #container_name,
                &{ use #krate::predicate::*; #pred_tokens },
                #pred_str,
                &[#(#target_str_lits),*],
            );
        }
    }

    /// Return expression for the single-target case (closure coercion or fn expr).
    /// Returns `None` for multi-target invocations (result type is `()`).
    fn return_expr(&self) -> Option<TokenStream> {
        if self.prepared.len() != 1 {
            return None;
        }
        match &self.prepared[0] {
            Target::Closure {
                arity,
                params,
                body,
                ..
            } => {
                let arg_tys: Vec<TokenStream> = (0..*arity).map(|_| quote! { _ }).collect();
                Some(quote! {
                    let __ir_assert_ret: fn(#(#arg_tys),*) -> _ = |#params| #body;
                    __ir_assert_ret
                })
            }
            Target::Function(expr) => Some(quote! { #expr }),
        }
    }
}

/// Shared code-generation entry point for both proc-macros.
///
/// `debug_only = false` → assert_ir!: container + assertion always emitted.
/// `debug_only = true`  → debug_assert_ir!: assertion gated on cfg(debug_assertions);
///                        multiple targets are a compile error outside debug mode.
fn codegen(input: TokenStream, debug_only: bool) -> TokenStream {
    let parsed: Punctuated<Expr, Token![,]> = match Punctuated::parse_terminated.parse2(input.clone()) {
        Ok(p) => p,
        Err(e) => abort!(e.span(), "ir-assert: parse error: {}", e),
    };

    let mut iter = parsed.iter();
    let crate_expr = iter
        .next()
        .unwrap_or_else(|| abort_call_site!("ir-assert: expected crate path"));
    let predicate_expr = iter
        .next()
        .unwrap_or_else(|| abort_call_site!("ir-assert: expected predicate expression"));
    let targets: Vec<&Expr> = iter.collect();

    if targets.is_empty() {
        abort_call_site!("ir-assert: expected at least one target function/closure after the predicate");
    }

    // Abort at proc-macro time for multi-target debug_assert_ir! in non-debug builds.
    // The IR-generation pass sets IR_ASSERT_IR_GEN to suppress this error.
    if debug_only && targets.len() > 1 && !debug_assertions_active() && std::env::var("IR_ASSERT_IR_GEN").is_err() {
        abort!(
            quote! { #(#targets)* },
            "debug_assert_ir! does not support multiple targets when debug_assertions is disabled"
        );
    }

    let cg = CodegenInput::parse(crate_expr, predicate_expr, &targets);
    let container_fn = cg.container_fn();
    let call = cg.macro_internal_call();
    let return_tokens = cg.return_expr().unwrap_or_default();

    quote! {
        {
            #container_fn
            #(if debug_only) {
                #[cfg(debug_assertions)]
                { #call }
            }
            #(else) {
                #call
            }
            #return_tokens
        }
    }
}

/// Returns true when `debug_assertions` is active for the crate being compiled.
///
/// `CARGO_CFG_DEBUG_ASSERTIONS` is only set for build scripts, not for proc-macros.
/// Instead we inspect rustc's own command-line args (the proc-macro runs inside rustc):
///
/// 1. An explicit `-C debug-assertions=yes/no` flag overrides everything.
/// 2. Otherwise, `debug_assertions` mirrors `opt-level`: it is ON when opt-level == 0
///    (the default) and OFF when opt-level > 0.
fn debug_assertions_active() -> bool {
    let args: Vec<String> = std::env::args().collect();

    // Helper: extract the value portion of a -C key=value flag.
    let cflag_val = |key: &str| -> Option<String> {
        for w in args.windows(2) {
            if w[0] == "-C" {
                if let Some(v) = w[1].strip_prefix(key) {
                    return Some(v.to_owned());
                }
            }
        }
        for a in &args {
            if let Some(v) = a.strip_prefix(&format!("-C{key}")) {
                return Some(v.to_owned());
            }
        }
        None
    };

    // Explicit -C debug-assertions= overrides everything.
    if let Some(val) = cflag_val("debug-assertions=") {
        return val == "yes" || val == "1";
    }

    // Derive from opt-level: debug_assertions is on only at opt-level 0 (the default).
    matches!(cflag_val("opt-level=").as_deref(), None | Some("0"))
}

#[proc_macro_error]
#[proc_macro]
pub fn __assert_ir_impl(input: TokenStream1) -> TokenStream1 {
    codegen(input.into(), false).into()
}

#[proc_macro_error]
#[proc_macro]
pub fn __debug_assert_ir_impl(input: TokenStream1) -> TokenStream1 {
    codegen(input.into(), true).into()
}
