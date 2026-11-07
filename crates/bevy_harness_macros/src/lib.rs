//! `#[bevy_harness::test]`: a test handed a ready [`TestApp`], built by a
//! setup function, and failed if it runs too long. Re-exported by
//! `bevy_harness`, whose documentation it is.
//!
//! [`TestApp`]: https://docs.rs/bevy_harness

use std::time::Duration;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    Attribute, Error, Expr, ExprArray, ExprLit, FnArg, ItemFn, Lit, Meta, ReturnType, Safety,
    Token, Type, UnOp, parse_macro_input,
};

/// A test handed what its setup builds, by `&mut`, with a timeout.
///
/// What it's handed is its app (`fn name(app: &mut TestApp)`), or a
/// fixture holding the app and whatever else the test needs by name (`fn
/// name(raid: &mut Raid)`); or nothing at all. It returns nothing, or a
/// `Result<(), E>`, failing on an error; it's a plain, safe, non-generic
/// `fn`, and its other attributes are kept (not another `#[test]`).
///
/// - `setup = ..`: what builds it. A function's path (`setup = server`) is
///   called; any other expression (`setup = physical(&["####"])`) is what it
///   builds, built in the test. Bevy's minimal app
///   (`TestApp::builder().build()`) if left out.
/// - `plugins = [PhysicsPlugin, ..]`: the minimal app with these plugins added,
///   for a test that needs nothing else; not with `setup`, which adds its own.
/// - `timeout = 5` (seconds, `0.5` too) or any `Duration` expression: fails the
///   test if it hasn't finished by then, instead of hanging; none if left out.
///   The test runs on a thread of its own then, so what setup builds has to be
///   `Send`. Past its timeout, the app stops at its next tick; a thread blocked
///   outside a tick can't be stopped, and is left running.
///
/// ```ignore
/// #[bevy_harness::test(setup = testing::server, timeout = 5)]
/// fn a_grip_lets_go(app: &mut TestApp) { .. }
/// ```
#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
    let options = parse_macro_input!(attr as Options);
    let body = parse_macro_input!(item as ItemFn);
    expand(options, body)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// What the attribute is given.
#[derive(Default)]
struct Options {
    /// What builds the app: a function's path, or the app itself.
    setup: Option<Expr>,
    /// The plugins the minimal app is built with, without a setup.
    plugins: Option<ExprArray>,
    /// How long it may run: a `Duration` expression.
    timeout: Option<proc_macro2::TokenStream>,
}

impl Parse for Options {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut options = Self::default();
        for meta in Punctuated::<Meta, Token![,]>::parse_terminated(input)? {
            let Meta::NameValue(pair) = &meta else {
                return Err(Error::new_spanned(
                    meta,
                    "expected `setup = ..`, `plugins = [..]` or `timeout = ..`",
                ));
            };
            if pair.path.is_ident("setup") {
                options.setup = Some(pair.value.clone());
            } else if pair.path.is_ident("plugins") {
                let Expr::Array(plugins) = &pair.value else {
                    return Err(Error::new_spanned(
                        &pair.value,
                        "expected a list of plugins: `[PhysicsPlugin, ..]`",
                    ));
                };
                options.plugins = Some(plugins.clone());
            } else if pair.path.is_ident("timeout") {
                options.timeout = Some(duration(&pair.value)?);
            } else {
                return Err(Error::new_spanned(
                    &pair.path,
                    "expected `setup`, `plugins` or `timeout`",
                ));
            }
        }
        Ok(options)
    }
}

/// The code building a timeout's `Duration`: a number is seconds, checked
/// here, so one that isn't more than 0 doesn't compile; anything else is a
/// `Duration` already.
fn duration(expr: &Expr) -> syn::Result<proc_macro2::TokenStream> {
    let Some(seconds) = seconds(expr)? else {
        return Ok(quote!(#expr));
    };
    let nanos = Duration::try_from_secs_f64(seconds)
        .ok()
        .and_then(|length| u64::try_from(length.as_nanos()).ok())
        .filter(|&nanos| nanos > 0)
        .ok_or_else(|| Error::new_spanned(expr, "expected a timeout in seconds, more than 0"))?;
    Ok(quote!(::core::time::Duration::from_nanos(#nanos)))
}

/// `expr` as a number, if it's a number literal (negated or not).
fn seconds(expr: &Expr) -> syn::Result<Option<f64>> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Float(lit),
            ..
        }) => lit.base10_parse().map(Some),
        Expr::Lit(ExprLit {
            lit: Lit::Int(lit), ..
        }) => lit.base10_parse().map(Some),
        Expr::Unary(unary) if matches!(unary.op, UnOp::Neg(_)) => {
            Ok(seconds(&unary.expr)?.map(|seconds| -seconds))
        }
        Expr::Paren(inner) => seconds(&inner.expr),
        Expr::Group(inner) => seconds(&inner.expr),
        _ => Ok(None),
    }
}

/// Rejects what the expansion can't run, each with an error saying why.
fn check(body: &ItemFn) -> syn::Result<()> {
    let sig = &body.sig;
    if let Some(asyncness) = sig.asyncness {
        return Err(Error::new_spanned(
            asyncness,
            "expected a plain `fn`: an app is ticked, not awaited",
        ));
    }
    if let Safety::Unsafe(unsafety) = sig.safety {
        return Err(Error::new_spanned(unsafety, "expected a safe `fn`"));
    }
    if let Some(constness) = sig.constness {
        return Err(Error::new_spanned(
            constness,
            "expected a plain `fn`, not `const`",
        ));
    }
    if let Some(abi) = &sig.abi {
        return Err(Error::new_spanned(
            abi,
            "expected a plain `fn`, not `extern`",
        ));
    }
    if !sig.generics.params.is_empty() || sig.generics.where_clause.is_some() {
        return Err(Error::new_spanned(
            &sig.generics,
            "expected no generics: a test is called with what setup builds",
        ));
    }
    if let Some(variadic) = &sig.variadic {
        return Err(Error::new_spanned(variadic, "expected no `...`"));
    }
    if sig.inputs.len() > 1 {
        return Err(Error::new_spanned(
            &sig.inputs,
            "expected one argument, what setup builds (`app: &mut TestApp`), or none",
        ));
    }
    if let Some(input) = sig.inputs.first() {
        let by_mut = matches!(
            input,
            FnArg::Typed(typed) if matches!(&*typed.ty, Type::Reference(by) if by.mutability.is_some())
        );
        if !by_mut {
            return Err(Error::new_spanned(
                input,
                "expected what setup builds by `&mut`: `app: &mut TestApp`",
            ));
        }
    }
    if let Some(test) = body.attrs.iter().find(|attr| is_test(attr)) {
        return Err(Error::new_spanned(
            test,
            "it's a test already: `#[bevy_harness::test]` adds the `#[test]`",
        ));
    }
    Ok(())
}

/// Whether `attr` makes a test of its own: `#[test]`, `#[tokio::test]`.
fn is_test(attr: &Attribute) -> bool {
    attr.path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "test")
}

/// Whether `output` is a `Result`, by its name: `io::Result<()>` too.
fn returns_result(output: &ReturnType) -> syn::Result<bool> {
    let ReturnType::Type(_, ty) = output else {
        return Ok(false);
    };
    let result = matches!(
        &**ty,
        Type::Path(path) if path.qself.is_none()
            && path.path.segments.last().is_some_and(|segment| segment.ident == "Result")
    );
    let unit = matches!(&**ty, Type::Tuple(tuple) if tuple.elems.is_empty());
    if result || unit {
        Ok(result)
    } else {
        Err(Error::new_spanned(
            ty,
            "expected no return type, or a `Result<(), E>`",
        ))
    }
}

/// The test: its body as a function of the app, run on the app `setup`
/// builds, within its timeout.
fn expand(options: Options, mut body: ItemFn) -> syn::Result<proc_macro2::TokenStream> {
    check(&body)?;
    let result = returns_result(&body.sig.output)?;
    let name = body.sig.ident.clone();
    let attrs = std::mem::take(&mut body.attrs);
    let vis = body.vis.clone();
    body.sig.ident = syn::Ident::new("body", name.span());
    let called = if body.sig.inputs.is_empty() {
        quote!(body())
    } else {
        quote!(body(it))
    };
    let call = if result {
        quote!(|it: &mut _| ::bevy_harness::__private::ok(#called))
    } else {
        quote!(|it: &mut _| #called)
    };
    let setup = match (options.setup, options.plugins) {
        (Some(_), Some(plugins)) => {
            return Err(Error::new_spanned(
                plugins,
                "`plugins` builds the minimal app: with `setup`, add them there",
            ));
        }
        (Some(Expr::Path(path)), None) => quote!(#path),
        (Some(app), None) => quote!(move || #app),
        (None, Some(plugins)) => {
            let plugins = plugins.elems.iter();
            quote!(|| ::bevy_harness::TestApp::builder().plugins((#(#plugins,)*)).build())
        }
        (None, None) => quote!(|| ::bevy_harness::TestApp::builder().build()),
    };
    let run = if let Some(timeout) = options.timeout {
        quote!(::bevy_harness::__private::run_with_timeout(#setup, #call, #timeout))
    } else {
        quote!(::bevy_harness::__private::run(#setup, #call))
    };
    Ok(quote! {
        #[::core::prelude::v1::test]
        #(#attrs)*
        #vis fn #name() {
            #body
            #run;
        }
    })
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::duration;

    fn timeout(expr: syn::Expr) -> String {
        match duration(&expr) {
            Ok(code) => code.to_string(),
            Err(error) => error.to_string(),
        }
    }

    #[test]
    fn a_number_is_seconds() {
        assert!(timeout(parse_quote!(5)).contains("from_nanos (5000000000u64)"));
        assert!(timeout(parse_quote!(0.25)).contains("from_nanos (250000000u64)"));
    }

    #[test]
    fn anything_else_is_a_duration() {
        assert_eq!(
            timeout(parse_quote!(Duration::from_millis(500))),
            "Duration :: from_millis (500)"
        );
    }

    #[test]
    fn a_timeout_has_to_be_more_than_nothing() {
        let expected = "expected a timeout in seconds, more than 0";
        assert_eq!(timeout(parse_quote!(0)), expected);
        assert_eq!(timeout(parse_quote!(-1.5)), expected);
    }
}
