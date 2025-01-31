

extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, Expr, Lit, Meta, Token, ExprLit, Error};
use syn::punctuated::Punctuated;
// #[proc_macro_attribute]
// pub fn tv_test(attr: TokenStream, item: TokenStream) -> TokenStream {
//     let test_vector = parse_macro_input!(attr as Expr);
//
//     let input_fn = parse_macro_input!(item as ItemFn);
//     let fn_vis = input_fn.vis;
//     let fn_name = input_fn.sig.ident;
//     let fn_block = input_fn.block;
//     let fn_inputs = input_fn.sig.inputs;
//
//     if fn_inputs.len() != 1 {
//         panic!("Function must have exactly one parameter");
//     }
//
//     let impl_fn_name = syn::Ident::new(
//         &format!("{}_test_impl", fn_name),
//         fn_name.span()
//     );
//
//     let expanded = quote! {
//         // Original function (renamed)
//         #fn_vis fn #impl_fn_name(#fn_inputs) {
//             #fn_block
//         }
//
//         #[test]
//         #fn_vis fn #fn_name() {
//             let test_cases = #test_vector;
//             for input in test_cases {
//                 #impl_fn_name(input);
//             }
//         }
//     };
//
//     TokenStream::from(expanded)
// }

#[proc_macro_attribute]
pub fn test_vec(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr with Punctuated::<Meta, Token![,]>::parse_terminated);
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let fn_block = &input.block;

    let mut file_path: Option<String> = None;
    let mut file_format: assert_tv::TestVectorFileFormat = assert_tv::TestVectorFileFormat::Yaml;
    let mut file_format_quoted = quote! {assert_tv::TestVectorFileFormat::Yaml};
    let mut test_mode = quote! { assert_tv::TestMode::from_environment() };
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
                }
            }
            _ => return Error::new_spanned(meta, "unsupported attribute format")
                .to_compile_error()
                .into(),
        }
    }
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
        #[test]
        fn #fn_name() {
            let _guard = assert_tv::initialize_tv_case_from_file(#file_path, #file_format_quoted, #test_mode)
                .expect("Error initializing test vector case");;
            #fn_block
            assert_tv::finalize_tv_case().expect("Error finalizing test vector case");
            drop(_guard)
        }
    };

    TokenStream::from(expanded)
}