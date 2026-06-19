// gluxe-macros — the `#[gluxe::command]` attribute.
//
// Turns a typed Rust function into a plugin command, generating the JSON
// argument deserialization and return serialization that would otherwise be
// hand-written against `serde_json::Value`. The generated code only references
// `::gluxe::...`, so this crate has no dependency on `gluxe` itself (which would
// be a cycle).
//
// See `crates/core/src/macro_support.rs` for the runtime half (`CommandSpec`,
// `extract`, `IntoCommandResult`) and the `commands!` helper that collects the
// generated specs into a `PluginBuilder`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    FnArg, Ident, ItemFn, LitStr, Pat, PatType, Token, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

/// Which command flavour the function registers as.
enum Kind {
    /// Inline on the Boa main thread (CPU-light work only).
    Sync,
    /// On a GPUI background thread; single result (I/O). Body must be `Send`.
    Async,
    /// On a GPUI background thread; many chunks via a final `StreamSink` param.
    Stream,
}

/// Parsed `#[command(...)]` arguments: `sync` | `async` | `stream` | `name = "..."`.
struct Args {
    kind: Kind,
    name: Option<String>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // `None` means "not yet specified", so a second flavour flag is a
        // conflict rather than a silent override (`#[command(async, stream)]`).
        let mut kind: Option<Kind> = None;
        let mut name: Option<String> = None;

        // Record a flavour flag, rejecting a second one. `span` points at the
        // offending token so the error underlines it.
        let set_kind = |kind: &mut Option<Kind>, new: Kind, span: proc_macro2::Span| {
            if kind.is_some() {
                return Err(syn::Error::new(
                    span,
                    "conflicting #[command] flavour: specify at most one of \
                     `sync`, `async`, `stream`",
                ));
            }
            *kind = Some(new);
            Ok(())
        };

        while !input.is_empty() {
            // `async` is a reserved keyword, so it never matches the `Ident`
            // branch below — peek for it explicitly.
            if input.peek(Token![async]) {
                let span = input.span();
                input.parse::<Token![async]>()?;
                set_kind(&mut kind, Kind::Async, span)?;
            } else {
                let id: Ident = input.parse()?;
                match id.to_string().as_str() {
                    "sync" => set_kind(&mut kind, Kind::Sync, id.span())?,
                    "stream" => set_kind(&mut kind, Kind::Stream, id.span())?,
                    "name" => {
                        input.parse::<Token![=]>()?;
                        let lit: LitStr = input.parse()?;
                        if name.is_some() {
                            return Err(syn::Error::new(
                                lit.span(),
                                "duplicate #[command] `name` argument",
                            ));
                        }
                        name = Some(lit.value());
                    }
                    other => {
                        return Err(syn::Error::new(
                            id.span(),
                            format!(
                                "unknown #[command] argument `{other}`; \
                                 expected `sync`, `async`, `stream`, or `name = \"...\"`"
                            ),
                        ));
                    }
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }
        Ok(Args {
            kind: kind.unwrap_or(Kind::Sync),
            name,
        })
    }
}

/// Mark a function as a gluxe plugin command. The function keeps its original
/// signature and body; the macro adds a sibling hidden module (same name, in the
/// type namespace, so it coexists with the function) exposing `__spec()` for the
/// `commands!` helper.
///
/// ```ignore
/// #[command(async, name = "readTextFile")]
/// fn read_text_file(path: String) -> Result<String, String> {
///     std::fs::read_to_string(&path).map_err(|e| e.to_string())
/// }
/// ```
#[proc_macro_attribute]
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as Args);
    let func = parse_macro_input!(item as ItemFn);
    expand(args, func)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(args: Args, func: ItemFn) -> syn::Result<TokenStream2> {
    let fn_name = &func.sig.ident;
    let vis = &func.vis;
    let cmd_name = args.name.unwrap_or_else(|| fn_name.to_string());

    // Collect the parameters that are deserialized from the JS args object. For
    // a stream command the trailing `StreamSink` parameter is excluded.
    let inputs: Vec<&FnArg> = func.sig.inputs.iter().collect();
    let total = inputs.len();
    let stream = matches!(args.kind, Kind::Stream);
    let mut sink_present = false;
    let mut idents: Vec<Ident> = Vec::new();
    let mut tys: Vec<Type> = Vec::new();
    let mut names: Vec<LitStr> = Vec::new();

    for (i, arg) in inputs.iter().enumerate() {
        let FnArg::Typed(PatType { pat, ty, .. }) = arg else {
            return Err(syn::Error::new_spanned(
                arg,
                "#[command] functions cannot take `self`",
            ));
        };
        if stream && i + 1 == total {
            // Final parameter of a stream command: the sink, passed through
            // verbatim rather than deserialized.
            sink_present = true;
            continue;
        }
        let Pat::Ident(pat_ident) = &**pat else {
            return Err(syn::Error::new_spanned(
                pat,
                "#[command] parameters must be simple identifiers",
            ));
        };
        names.push(LitStr::new(
            &pat_ident.ident.to_string(),
            pat_ident.ident.span(),
        ));
        idents.push(pat_ident.ident.clone());
        tys.push((**ty).clone());
    }

    if stream && !sink_present {
        return Err(syn::Error::new_spanned(
            &func.sig,
            "#[command(stream)] functions must take a final `gluxe::StreamSink` parameter",
        ));
    }

    // Name the args parameter `_args` when nothing is extracted from it, so the
    // generated wrapper does not trip the unused-variable lint.
    let args_param = if idents.is_empty() {
        quote!(_args)
    } else {
        quote!(args)
    };

    let body = match args.kind {
        Kind::Sync | Kind::Async => {
            let ctor = if matches!(args.kind, Kind::Async) {
                quote!(async_)
            } else {
                quote!(sync)
            };
            quote! {
                fn __wrapper(#args_param: ::gluxe::__macro::Value) -> ::gluxe::CommandResult {
                    #( let #idents: #tys = ::gluxe::__macro::extract(&args, #names)?; )*
                    ::gluxe::__macro::IntoCommandResult::into_command_result(
                        super::#fn_name(#(#idents),*)
                    )
                }
                pub fn __spec() -> ::gluxe::__macro::CommandSpec {
                    ::gluxe::__macro::CommandSpec::#ctor(#cmd_name, __wrapper)
                }
            }
        }
        Kind::Stream => quote! {
            fn __wrapper(#args_param: ::gluxe::__macro::Value, mut __sink: ::gluxe::StreamSink) {
                #(
                    let #idents: #tys = match ::gluxe::__macro::extract(&args, #names) {
                        ::core::result::Result::Ok(v) => v,
                        ::core::result::Result::Err(e) => { __sink.error(e); return; }
                    };
                )*
                super::#fn_name(#(#idents,)* __sink);
            }
            pub fn __spec() -> ::gluxe::__macro::CommandSpec {
                ::gluxe::__macro::CommandSpec::stream(#cmd_name, __wrapper)
            }
        },
    };

    Ok(quote! {
        #func

        #[doc(hidden)]
        #vis mod #fn_name {
            #[allow(unused_imports)]
            use super::*;
            #body
        }
    })
}

// ---------------------------------------------------------------------------
// Tests for the `#[command(...)]` argument parser.
//
// `Args: Parse` only touches `syn`/`proc-macro2` types, so it can be exercised
// directly via `syn::parse_str` (no `proc_macro` token stream required).
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::{Args, Kind};

    // `Args` has no `Debug` impl, so `Result::unwrap`/`unwrap_err` are unavailable;
    // unwrap via explicit matches instead.
    fn parse_ok(src: &str) -> Args {
        match syn::parse_str::<Args>(src) {
            Ok(a) => a,
            Err(e) => panic!("expected ok for {src:?}, got error: {e}"),
        }
    }

    fn parse_err(src: &str) -> syn::Error {
        match syn::parse_str::<Args>(src) {
            Ok(_) => panic!("expected parse error for {src:?}"),
            Err(e) => e,
        }
    }

    #[test]
    fn conflicting_flavours_is_error() {
        let err = parse_err("async, stream");
        assert!(
            err.to_string().contains("conflicting"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn name_only_defaults_to_sync() {
        let args = parse_ok("name = \"readFile\"");
        assert_eq!(args.name.as_deref(), Some("readFile"));
        assert!(matches!(args.kind, Kind::Sync));
    }

    #[test]
    fn async_with_name() {
        let args = parse_ok("async, name = \"x\"");
        assert!(matches!(args.kind, Kind::Async));
        assert_eq!(args.name.as_deref(), Some("x"));
    }

    #[test]
    fn duplicate_name_is_error() {
        let err = parse_err("name=\"a\", name=\"b\"");
        assert!(
            err.to_string().contains("duplicate"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn unknown_arg_is_error_with_expected_list() {
        let err = parse_err("bogus");
        let msg = err.to_string();
        assert!(msg.contains("unknown #[command] argument"), "msg: {msg}");
        assert!(msg.contains("name = \"...\""), "msg: {msg}");
    }

    #[test]
    fn empty_args_default_sync_no_name() {
        let args = parse_ok("");
        assert!(matches!(args.kind, Kind::Sync));
        assert_eq!(args.name, None);
    }
}
