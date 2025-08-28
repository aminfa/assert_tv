mod derive;

extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, DeriveInput, Error, Expr, ItemFn, Lit, Meta, Token};

/// Derive `assert_tv::TestVectorSet` for a struct of `TestValue<…>` fields.
///
/// Requirements:
/// - Struct with named fields (no tuple/unit structs).
/// - Every field is of type `assert_tv::TestValue<…>`.
///
/// Per-field attributes via `#[test_vec(...)]`:
/// - `name = "…"`: human-readable field name.
/// - `description = "…"`: longer description for reports.
/// - `serialize_with = "path::to::fn"`: `fn(&T) -> anyhow::Result<serde_json::Value>`.
/// - `deserialize_with = "path::to::fn"`: `fn(&serde_json::Value) -> anyhow::Result<T>`.
/// - `offload = true`: store value in a compressed sidecar file.
///
/// Example:
/// ```rust,ignore
/// use assert_tv::{TestVectorSet, TestValue};
///
/// #[derive(TestVectorSet)]
/// struct Fields {
///     #[test_vec(name = "rand", description = "random input")]
///     rand: TestValue<u64>,
///
///     #[test_vec(name = "out", offload = true)]
///     out: TestValue<Vec<u8>>,
/// }
/// ```
#[proc_macro_derive(TestVectorSet, attributes(test_vec))]
pub fn derive_test_vector_set(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match crate::derive::expand(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Attribute macro for tests that use assert_tv test vectors.
///
/// Wraps a `#[test]` function with automatic initialization/finalization of a
/// test‑vector session. Controls the file path, format, and mode.
///
/// Arguments:
/// - `file = "path/to/file.ext"` (optional): defaults to `.test_vectors/<fn_name>.<format>`.
/// - `format = "json" | "yaml" | "toml"` (optional): defaults to `"json"`.
/// - `mode = "init" | "check"` (optional): defaults to `TEST_MODE` env var, else `"check"`.
///
/// Example:
/// ```rust,ignore
/// use assert_tv_macros::test_vec_case;
///
/// #[test_vec_case]
/// fn my_default_case() {
///     // uses .test_vectors/my_default_case.json
/// }
///
/// #[test_vec_case(file = "tests/vecs/case.yaml", format = "yaml", mode = "init")]
/// fn my_yaml_init_case() {
///     // initializes YAML vectors at the given path
/// }
/// ```
#[proc_macro_attribute]
pub fn test_vec_case(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr with Punctuated::<Meta, Token![,]>::parse_terminated);
    let input = parse_macro_input!(item as ItemFn);
    let fn_result = &input.sig.output;
    let fn_name = &input.sig.ident;
    let fn_block = &input.block;

    let mut file_path: Option<String> = None;
    let mut file_format_ending: &'static str = "json";
    let mut file_format_quoted = quote! {assert_tv::TestVectorFileFormat::Json};
    let mut test_mode = quote! { assert_tv::TestMode::from_environment() };

    // Process attribute arguments
    for meta in args {
        match meta {
            Meta::NameValue(nv) => {
                let ident = nv.path.get_ident().unwrap_or_else(|| {
                    panic!("Invalid attribute argument");
                });

                if ident == "file" {
                    if let Expr::Lit(lit_str) = nv.value {
                        let Lit::Str(v) = lit_str.lit else {
                            return Error::new_spanned(lit_str, "expected string literal")
                                .to_compile_error()
                                .into();
                        };
                        file_path = Some(v.value());
                    } else {
                        return Error::new_spanned(nv.value, "expected string literal")
                            .to_compile_error()
                            .into();
                    }
                } else if ident == "format" {
                    if let Expr::Lit(lit_str) = nv.value {
                        let Lit::Str(v) = lit_str.clone().lit else {
                            return Error::new_spanned(lit_str.clone(), "expected string literal")
                                .to_compile_error()
                                .into();
                        };
                        (file_format_ending, file_format_quoted) = match v.value().as_str() {
                            "yaml" | "yml" => {
                                ("yaml", quote! {assert_tv::TestVectorFileFormat::Yaml})
                            }
                            "json" => ("json", quote! {assert_tv::TestVectorFileFormat::Json}),
                            "toml" => ("toml", quote! {assert_tv::TestVectorFileFormat::Toml}),
                            _ => {
                                return Error::new_spanned(
                                    lit_str,
                                    "invalid format, expected yaml/yml or json",
                                )
                                .to_compile_error()
                                .into();
                            }
                        };
                    } else {
                        return Error::new_spanned(nv.value, "expected string literal")
                            .to_compile_error()
                            .into();
                    }
                } else if ident == "mode" {
                    if let Expr::Lit(lit_str) = nv.value {
                        let Lit::Str(v) = lit_str.clone().lit else {
                            return Error::new_spanned(lit_str.clone(), "expected string literal")
                                .to_compile_error()
                                .into();
                        };
                        test_mode = match v.value().as_str() {
                            "init" => quote! {assert_tv::TestMode::Init},
                            "check" => quote! {assert_tv::TestMode::Check},
                            _ => {
                                return Error::new_spanned(
                                    lit_str,
                                    "invalid format, expected init, check",
                                )
                                .to_compile_error()
                                .into();
                            }
                        };
                    } else {
                        return Error::new_spanned(nv.value, "expected string literal")
                            .to_compile_error()
                            .into();
                    }
                }
            }
            _ => {
                return Error::new_spanned(meta, "unsupported attribute format")
                    .to_compile_error()
                    .into()
            }
        }
    }
    let file_path: String = match file_path {
        Some(file_path) => file_path,
        None => {
            // default file path is derived from the test function name and is put under .test_vectors/
            let default_file = format!(".test_vectors/{fn_name}.{file_format_ending}");
            default_file
        }
    };

    // let file_path_quoted = quote! {file_path};
    // let file_format_quoted = quote! {file_format};
    let expanded = quote! {
        #[test]
        fn #fn_name() #fn_result {
            let _guard = assert_tv::initialize_tv_case_from_file(#file_path, #file_format_quoted, #test_mode)
                .expect("Error initializing test vector case");
            let result = #fn_block;
            assert_tv::finalize_tv_case().expect("Error finalizing test vector case");
            drop(_guard);
            result
        }
    };

    TokenStream::from(expanded)
}
