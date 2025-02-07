

extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, ItemFn, Expr, Lit, Meta, Token, Error, ReturnType};
use syn::punctuated::Punctuated;

/// An attribute macro for simplifying the creation of test functions that utilize test vectors.
///
/// This macro allows you to write data-driven tests using external test vectors stored in YAML or JSON files.
/// It provides flexibility by supporting different modes of operation: initializing, checking, and recording test cases.
/// 
/// # Usage
///
/// The `test_vec` macro is applied as an attribute to test functions. Here's a basic example:
///
/// ```rust
/// use assert_tv_macros::test_vec;
/// #[test_vec(feature="tv")]
/// fn my_test() {
///     // Test code here
/// }
/// ```
///
/// ## Arguments
///
/// The `test_vec` macro accepts the following arguments:
///
/// 1. **`file`**: Specifies the path to the test vector file.
///    - Format: `"path/to/file.{yaml|json}"`
///    - Example: `#[test_vec(file = "tests/vecs/my_test.yaml")]`
///    - Default: None (uses a default based on function name and format)
///
/// 2. **`format`**: Determines the format of the test vector file.
///    - Possible values:
///      - `"yaml"` or `"yml"`
///      - `"json"`
///    - Default: `"yaml"`
///    - Example: `#[test_vec(format = "json")]`
///
/// 3. **`mode`**: Specifies the test mode.
///    - Possible values:
///      - `"init"`
///      - `"check"`
///      - `"record"`
///    - Default: If `TEST_MODE` env-variable is defined, it will be used. Else `"check"` is used as the default.
///    - Example: `#[test_vec(mode = "debug")]`
/// 
/// # Notes
///
/// - The generated default file path for test vectors is `.test_vectors/<function_name>.<format>`.
/// - Test functions wrapped with this macro are marked as `#[ignore]` by default. To include them in test runs, use the `--ignored` flag.
/// - The macro automatically initializes and cleans up test vector resources, ensuring proper setup and teardown.
#[proc_macro_attribute]
pub fn test_vec(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr with Punctuated::<Meta, Token![,]>::parse_terminated);
    let input = parse_macro_input!(item as ItemFn);
    let fn_result = &input.sig.output;
    // let returns_result = match input.sig.output {
    //     ReturnType::Default => {}
    //     ReturnType::Type(_, _) => {}
    // }
    let fn_name = &input.sig.ident;
    let fn_block = &input.block;

    let mut file_path: Option<String> = None;
    let mut file_format: assert_tv::TestVectorFileFormat = assert_tv::TestVectorFileFormat::Yaml;
    let mut file_format_quoted = quote! {assert_tv::TestVectorFileFormat::Yaml};
    let mut test_mode = quote! { assert_tv::TestMode::from_environment() };
    let mut feature_flag: Option<String> = None;

    // Process attribute arguments
    for meta in args {
        match meta {
            Meta::NameValue(nv) => {
                let ident = nv.path.get_ident().unwrap_or_else(|| {
                    panic!("Invalid attribute argument");
                });

                if ident == "file" {
                    if let Expr:: Lit(lit_str)  = nv.value {
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
                    if let Expr:: Lit(lit_str)  = nv.value {
                        let Lit::Str(v) = lit_str.clone().lit else {
                            return Error::new_spanned(lit_str.clone(), "expected string literal")
                                .to_compile_error()
                                .into();
                        };
                        (file_format, file_format_quoted) = match v.value().as_str() {
                            "yaml" | "yml" => (assert_tv::TestVectorFileFormat::Yaml, 
                                               quote! {assert_tv::TestVectorFileFormat::Yaml}),
                            "json" => (assert_tv::TestVectorFileFormat::Json,
                                       quote! {assert_tv::TestVectorFileFormat::Json}),
                            _ => {
                                return Error::new_spanned(lit_str, "invalid format, expected yaml/yml or json")
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
                else if ident == "mode" {
                    if let Expr:: Lit(lit_str)  = nv.value {
                        let Lit::Str(v) = lit_str.clone().lit else {
                            return Error::new_spanned(lit_str.clone(), "expected string literal")
                                .to_compile_error()
                                .into();
                        };
                        test_mode = match v.value().as_str() {
                            "init" => quote! {assert_tv::TestMode::Init},
                            "check" => quote! {assert_tv::TestMode::Check},
                            "record" => quote! {assert_tv::TestMode::Record},
                            _ => {
                                return Error::new_spanned(lit_str, "invalid format, expected init, check or record")
                                    .to_compile_error()
                                    .into();
                            }
                        };
                    } else {
                        return Error::new_spanned(nv.value, "expected string literal")
                            .to_compile_error()
                            .into();
                    }
                } else if ident == "feature" {
                    // NEW: Parse the `feature` argument.
                    if let Expr::Lit(lit_expr) = nv.value {
                        let Lit::Str(v) = lit_expr.lit else {
                            return Error::new_spanned(lit_expr, "expected string literal for feature")
                                .to_compile_error()
                                .into();
                        };
                        feature_flag = Some(v.value());
                    } else {
                        return Error::new_spanned(nv.value, "expected string literal for feature")
                            .to_compile_error()
                            .into();
                    }
                }
            }
            _ => return Error::new_spanned(meta, "unsupported attribute format")
                .to_compile_error()
                .into(),
        }
    }
    let Some(feature_flag) = feature_flag else {
        return Error::new(Span::call_site(), "expected a feature flag, e.g. `feature = \"tv\"`")
            .to_compile_error()
            .into();
    };
    let file_path: String = match file_path {
        Some(file_path) => file_path,
        None => {
            // default file path is derived from the test function name and is put under .test_vectors/
            let file_ending = match file_format {
                assert_tv::TestVectorFileFormat::Yaml => "yaml",
                assert_tv::TestVectorFileFormat::Json => "json"
            };
            let default_file = format!(".test_vectors/{}.{}", fn_name, file_ending);
            default_file
        }
    };



    // let file_path_quoted = quote! {file_path};
    // let file_format_quoted = quote! {file_format};
    let expanded = quote! {
        #[cfg(feature=#feature_flag)]
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