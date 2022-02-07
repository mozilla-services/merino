use proc_macro::TokenStream;
use quote::quote;
use syn::{
    punctuated::Punctuated, token::Comma, Attribute, ExprClosure, FnArg, Pat, PatType, Path,
    PathArguments, PathSegment, TypePath,
};

/// Wrap a test case in setup and tear down boiler plate, including manipulating
/// settings if needed.
///
/// If other test macros are used, such as the [`parameterized`
/// crate](https://crates.io/crates/parameterized), the attributes for those
/// macros should be placed below the `merino_test` macro. Additionally, the
/// settings closure can specify arguments that will be available on the
/// resulting function, for use by `parameterized`.
///
/// # Example:
///
/// Basic usage:
///
/// ```
/// use merino_integration_tests::{merino_test_macro, TestingTools};
///
/// #[merino_test_macro]
/// async fn test_function(TestingTools { test_client, .. }: TestingTools) {
///     // test using test_client
/// }
/// ```
///
/// Settings can be customized:
///
/// ```
/// use merino_integration_tests::{merino_test_macro, TestingTools};
///
/// #[merino_test_macro(|settings| settings.debug = true)]
/// async fn test_function(TestingTools { test_client, .. }: TestingTools) {
///     // test using test_client while the debug setting is true.
/// }
/// ```
///
/// Other test macros, like `parameterized`, can be used:
///
/// ```
/// use merino_integration_tests::{TestingTools, merino_test_macro};
/// use parameterized::parameterized;
///
/// #[merino_test_macro(|settings, ttl: u64| settings.redis_cache.default_ttl = ttl)]
/// #[parameterized(ttl = { 300, 600 })]
/// async fn test(TestingTools { .. }: TestingTools) {
///     // test will run twice, once with each TTL setting.
/// }
/// ```
///
#[proc_macro_attribute]
pub fn merino_test(attributes: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the function that this macro is attached to.
    let mut input = syn::parse_macro_input!(item as syn::ItemFn);
    let attrs = &input.attrs;
    let vis = &input.vis;
    let sig = &mut input.sig;
    let body = &input.block;

    // Check that function is async. Then remove `async` from the signature,
    // so we can reuse the same signature for our blocking sync outer function.
    if sig.asyncness.is_none() {
        return syn::Error::new_spanned(
            input.sig.fn_token,
            "the async keyword is missing from the function declaration",
        )
        .to_compile_error()
        .into();
    }
    sig.asyncness = None;

    // Check if the `#[test]` attribute should be added.
    let missing_test_attr = maybe_add_test_attr(&attrs[..]);

    // A place to store all the top-level arguments along the following parsing.
    let mut toplevel_args = Punctuated::<FnArg, Comma>::new();

    // Parse the `TestingTools` argument.
    let testing_tools_arg = parse_testing_tools_arg(&sig.inputs, &mut toplevel_args);
    if testing_tools_arg.is_none() {
        return syn::Error::new_spanned(&sig.inputs, "expected an argument of type TestingTools")
            .into_compile_error()
            .into();
    }

    // Parse the body of the settings closure if any.
    let settings_body = if attributes.is_empty() {
        quote!({})
    } else {
        let mut settings_closure = syn::parse_macro_input!(attributes as ExprClosure);
        parse_settings_body(&mut settings_closure, &mut toplevel_args)
    };

    // Add all collected arguments to the top level signature.
    sig.inputs = toplevel_args;

    // output the built test function
    (quote! {
        #(#attrs)*
        #missing_test_attr
        #vis #sig {
            actix_rt::System::new()
                .block_on(async {
                    // crate here refers to `merino-integration-tests`
                    crate::merino_test(
                        |settings| { #settings_body },
                        | #testing_tools_arg | async move { #body }
                    ).await
                })
        }
    })
    .into()
}

/// Add the `#[test]` attribute to the test if needed.
///
/// This is mainly for those parameterized tests (`#[parameterized(...)]`) by
/// the crate `parameterized`, which unconditionally adds `#[test]`.
fn maybe_add_test_attr(attrs: &[Attribute]) -> proc_macro2::TokenStream {
    let has_test_attr = attrs.iter().any(|attr| attr.path.is_ident("test"));

    let is_parameterized = attrs.iter().any(|attr| {
        let segment_names: Vec<_> = attr
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        (segment_names == vec!["parameterized"])
            || (segment_names == vec!["parameterized", "parameterized"])
    });

    // If parameterized is not being used and there is no test attribute already
    // present, add one.
    if has_test_attr || is_parameterized {
        quote!()
    } else {
        quote!(#[test])
    }
}

/// Find and take the `TestingTools` argument from the arguments of the test
/// function, leaving any others in place by storing in `toplevel_args`.
///
///
/// # Example
///
/// Parse the `TestingTools { test_client, .. }: TestingTools` from the
/// following test.
///
/// ```ignore
/// #[merino_test_macro]
/// async fn test(TestingTools { test_client, .. }: TestingTools) {
///   ...
/// }
/// ```
///
/// This has the pattern match that defines the bindings the caller is
/// expecting to use, so we have to use it directly.
fn parse_testing_tools_arg(
    args: &Punctuated<FnArg, Comma>,
    toplevel_args: &mut Punctuated<FnArg, Comma>,
) -> Option<FnArg> {
    let mut testing_tools_arg = None;

    for arg in args {
        match arg {
            syn::FnArg::Receiver(_) => toplevel_args.push(arg.clone()),
            syn::FnArg::Typed(PatType { ref ty, .. }) => match &**ty {
                syn::Type::Path(TypePath {
                    qself: None,
                    path:
                        Path {
                            leading_colon: None,
                            segments,
                        },
                }) if !segments.is_empty() => {
                    let segment = segments.last().unwrap();
                    match segment {
                        PathSegment {
                            arguments: PathArguments::None,
                            ident,
                        } if *ident == "TestingTools" => {
                            testing_tools_arg = Some(arg.clone());
                        }
                        _ => toplevel_args.push(arg.clone()),
                    };
                }
                _ => toplevel_args.push(arg.clone()),
            },
        }
    }

    testing_tools_arg
}

/// Parse the body of the `settings` closure.
///
/// # Example
///
/// Parse the `settings.debug = false` from the following test.
///
/// ```ignore
/// #[merino_test_macro(|settings| settings.debug = false)]
/// async fn test(TestingTools { test_client, .. }: TestingTools) {
///   ...
/// }
/// ```
///
/// The settings closure in the macro invocation can also contribute
/// arguments. These arguments, stored in `toplevel_args`, can be used by other
/// macros like parameterized.
fn parse_settings_body(
    settings_closure: &mut ExprClosure,
    toplevel_args: &mut Punctuated<FnArg, Comma>,
) -> proc_macro2::TokenStream {
    let new_settings_args = Punctuated::<Pat, Comma>::new();
    for arg in &settings_closure.inputs {
        match arg {
            Pat::Ident(pat) if pat.ident == "settings" => (),
            Pat::Type(pat) => toplevel_args.push(FnArg::Typed(pat.clone())),
            _ => {
                return syn::Error::new_spanned(
                    arg,
                    "only `val: Type` parameters can be used for merino_test settings inputs",
                )
                .into_compile_error()
            }
        }
    }
    settings_closure.inputs = new_settings_args;
    let closure_body = &settings_closure.body;
    quote!({ #closure_body })
}
