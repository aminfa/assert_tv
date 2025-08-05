mod derive;

extern crate proc_macro;
use std::borrow::Borrow;

use proc_macro::{Ident, TokenStream};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, DeriveInput, Error, Expr, ExprLit, ItemFn, Lit, Meta, Token};

/// Derive macro for [`assert_tv::TestVectorSet`].
///
/// This macro generates an implementation of
/// ```text
/// impl assert_tv::TestVectorSet for YourStruct { … }
/// ```
/// whose `start::<TV>()` constructor populates each field with an
/// [`assert_tv::TestValue<T>`] pre‑initialised for the current source‑code
/// location.
///
/// ---
/// # Requirements
///
/// * **All struct fields must be of type** `TestValue<…>`.
/// * The struct must be **non‑tuple, non‑unit, with named fields.**
///
/// ---
/// # Field attributes
///
/// Each field may carry zero or more `#[test_vec(...)]` helpers:
///
/// | Key&nbsp; | Type&nbsp; | Meaning | Default if omitted |
/// |----------|-----------|---------|---------------------|
/// | `name`             | `&'static str` | Human‑readable display name. | `None` |
/// | `description`      | `&'static str` | Longer explanation for docs / reports. | `None` |
/// | `serialize_with`   | *path* to `fn(&T) -> anyhow::Result<serde_json::Value>` | Custom serializer | `serde_json::to_value`  |
/// | `deserialize_with` | *path* to `fn(&serde_json::Value) -> anyhow::Result<T>` | Custom deserializer | `serde_json::from_value` |
///
/// Multiple `#[test_vec]` attributes may be stacked on the same field:
///
/// ```rust,ignore
/// #[test_vec(name = "counter")]
/// #[test_vec(description = "u64 monotonically increasing")]
/// #[test_vec(serialize_with = "my_serialize")]
/// count: TestValue<u64>,
/// ```
///
/// Duplicate keys on the same field are rejected with a clear error
/// (`duplicate "name" key`, etc.).
///
/// ---
/// # Example
///
/// ```rust,ignore
/// use assert_tv::{TestVectorSet, TestValue};
///
/// #[derive(TestVectorSet)]
/// struct SomeTestFields {
///     #[test_vec(name = "a", description = "a is a u64")]
///     #[test_vec(serialize_with = "custom_serialize_fn")]
///     #[test_vec(deserialize_with = "custom_deserialize_fn")]
///     a: TestValue<u64>,
///
///     // No attributes: falls back to defaults.
///     b: TestValue<String>,
/// }
///
/// fn custom_serialize_fn(v: &u64) -> anyhow::Result<serde_json::Value> {
///     Ok(serde_json::json!(v))
/// }
///
/// fn custom_deserialize_fn(v: &serde_json::Value) -> anyhow::Result<u64> {
///     Ok(v.as_u64().unwrap_or_default())
/// }
/// ```
///
/// Running `cargo doc --open` after adding this comment will render the
/// table, example, and key descriptions in a readable, searchable format.
///
/// ---
#[proc_macro_derive(TestVectorSet, attributes(test_vec))]
pub fn derive_test_vector_set(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match crate::derive::expand(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// An attribute macro for simplifying the creation of test functions that utilize test vectors.
///
/// This macro allows you to write data-driven tests using external test vectors stored in YAML or JSON files.
/// It provides flexibility by supporting different modes of operation: initializing, checking, and recording test cases.
///
/// # Usage
///
/// The `test_vec_case` macro is applied as an attribute to test functions. Here's a basic example:
///
/// ```rust
/// use assert_tv_macros::test_vec_case;
/// #[test_vec_case]
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
///      - `"toml"`
///    - Default: `"yaml"`
///    - Example: `#[test_vec(format = "json")]`
///
/// 3. **`mode`**: Specifies the test mode.
///    - Possible values:
///      - `"init"`
///      - `"check"`
///    - Default: If no value is specified, the `TEST_MODE` env-variable is queried for a fall-back. Else `"check"` is used as the default.
///    - Example: `#[test_vec(mode = "init")]`
///
/// # Notes
///
/// - The generated default file path for test vectors is `.test_vectors/<function_name>.<format>`.
/// - Test functions wrapped with this macro are marked as `#[ignore]` by default. To include them in test runs, use the `--ignored` flag.
/// - The macro automatically initializes and cleans up test vector resources, ensuring proper setup and teardown.
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
        // We expect that the meta argument is a named value with a proper name; short-circuit otherwise!
        let nv = match meta.clone() {
            Meta::NameValue(nv) => nv,
            _ => {
                return Error::new_spanned(meta, "unsupported attribute format")
                    .to_compile_error()
                    .into()
            }
        };

        // We expect that the named value has a proper ident; short-circuit otherwise
        let ident = nv.path.get_ident().unwrap_or_else(|| {
            panic!("Invalid attribute argument");
        });

        match (ident.to_string().borrow(), &nv.value) {
            ("file", Expr::Lit(ExprLit { lit: Lit::Str(v), .. })) => {
                file_path = Some(v.value());
            },

            ("format", Expr::Lit(lit_str @ ExprLit { lit: Lit::Str(val), .. })) => { 
                (file_format_ending, file_format_quoted) = match val.value().as_str() {
                    "yaml" | "yml" => ("yaml", quote! {assert_tv::TestVectorFileFormat::Yaml}),
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
            },

            ("mode", Expr::Lit(lit_str @ ExprLit { lit: Lit::Str(val), .. })) => { 
                test_mode = match val.value().as_str() {
                    "init" => quote! {assert_tv::TestMode::Init},
                    "check" => quote! {assert_tv::TestMode::Check},
                    _ => {
                        return Error::new_spanned(lit_str, "invalid format, expected init, check")
                            .to_compile_error()
                            .into();
                    }
                };
            }

            ("file" | "format" | "mode", nv_value) => {
                return Error::new_spanned(nv_value, "expected string literal")
                    .to_compile_error()
                    .into();
            },

            _ => return Error::new_spanned(meta, "unsupported attribute format")
                .to_compile_error()
                .into(),
        }
    }
    let file_path: String = match file_path {
        Some(file_path) => file_path,
        None => {
            // default file path is derived from the test function name and is put under .test_vectors/
            let default_file = format!(".test_vectors/{}.{}", fn_name, file_format_ending);
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

