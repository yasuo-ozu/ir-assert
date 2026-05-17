use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Span, TokenStream};
use std::sync::atomic::{AtomicU64, Ordering};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::*;
use template_quote::{quote, ToTokens};

/// Global counter to ensure each macro invocation produces a unique container name,
/// even when the same predicate+targets pair appears multiple times.
/// This is deterministic across compilations because macro expansion order is stable.
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique hash from the macro input tokens and an invocation counter.
fn unique_hash(input: &TokenStream) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    let s = input.to_string();
    let normalized: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    normalized.hash(&mut hasher);
    COUNTER.fetch_add(1, Ordering::Relaxed).hash(&mut hasher);
    hasher.finish()
}

/// Prepare a target expression for codegen. Returns the statements to embed in the container.
enum Target<'a> {
    Closure {
        coerce_ident: Ident,
        arity: usize,
        params: &'a Punctuated<Pat, Token![,]>,
        body: &'a Expr,
    },
    Function(&'a Expr),
}

fn inner(input: TokenStream) -> TokenStream {
    let parsed: Punctuated<Expr, Token![,]> = Punctuated::parse_terminated
        .parse2(input.clone())
        .unwrap_or_else(|e| panic!("ir-assert: parse error: {}", e));

    let mut iter = parsed.iter();
    let crate_expr = iter
        .next()
        .unwrap_or_else(|| panic!("ir-assert: expected crate path"));
    let predicate_expr = iter
        .next()
        .unwrap_or_else(|| panic!("ir-assert: expected predicate expression"));
    let targets: Vec<&Expr> = iter.collect();

    if targets.is_empty() {
        panic!("ir-assert: expected at least one target function/closure after the predicate");
    }

    let krate: TokenStream = quote! { #crate_expr };

    // Hash predicate + targets for deterministic naming
    let hash_input: TokenStream = {
        let pred_ts: TokenStream = quote! { #predicate_expr };
        let targets_ts: Vec<TokenStream> = targets.iter().map(|t| quote! { #t }).collect();
        quote! { #pred_ts #(#targets_ts)* }
    };
    let r = unique_hash(&hash_input);

    let container_name = format!("ir_assert_container_{}", r);
    let container_ident = Ident::new(&container_name, Span::call_site());

    // Stringify the predicate and targets for error messages (before transformation)
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
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");

    let args: Vec<String> = std::env::args().collect();
    let is_test = args.iter().any(|a| a == "--test");
    let crate_name = args
        .iter()
        .position(|a| a == "--crate-name")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    let asm_tag = LitStr::new(&format!("/* ir_assert {} {{0}} */", r), Span::call_site());

    // Prepare targets as an enum for codegen
    let prepared: Vec<Target> = targets
        .iter()
        .enumerate()
        .map(|(i, target)| {
            if let Expr::Closure(closure) = target {
                Target::Closure {
                    coerce_ident: Ident::new(&format!("__ir_assert_fn_{}", i), Span::call_site()),
                    arity: closure.inputs.len(),
                    params: &closure.inputs,
                    body: &closure.body,
                }
            } else {
                Target::Function(target)
            }
        })
        .collect();

    let target_stmts: Vec<TokenStream> = prepared
        .iter()
        .map(|t| match t {
            Target::Closure {
                coerce_ident,
                arity,
                params,
                body,
            } => {
                let arg_tys: Vec<TokenStream> = (0..*arity).map(|_| quote! { _ }).collect();
                quote! {
                    let #coerce_ident: fn(#(#arg_tys),*) -> _ = |#params| #body;
                    unsafe {
                        core::arch::asm!(#asm_tag, in(reg) #coerce_ident as usize,
                            options(nostack, preserves_flags, readonly));
                    }
                }
            }
            Target::Function(expr) => quote! {
                unsafe {
                    core::arch::asm!(#asm_tag, in(reg) #expr as usize,
                        options(nostack, preserves_flags, readonly));
                }
            },
        })
        .collect();

    quote! {
        {
            #[no_mangle]
            #[inline(never)]
            #[allow(unused, dead_code)]
            fn #container_ident() {
                #(#target_stmts)*
            }

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
}

#[proc_macro]
pub fn __assert_ir_impl(input: TokenStream1) -> TokenStream1 {
    inner(input.into()).into()
}
