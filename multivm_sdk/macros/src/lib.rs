use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ImplItem, ItemFn, ItemImpl, Type};

#[proc_macro_attribute]
pub fn contract(_: TokenStream, input: TokenStream) -> TokenStream {
    let quote_input: proc_macro2::TokenStream = input.clone().into();

    let methods = if let Ok(contract_impl) = syn::parse::<ItemImpl>(input) {
        let contract_name = if let Type::Path(path) = *contract_impl.self_ty {
            path.path.segments[0].ident.clone()
        } else {
            panic!("Invalid contract name")
        };
        contract_impl.items.iter().map(|impl_item| {
            if let ImplItem::Fn(ref method) = impl_item {
                let method_name = method.sig.ident.clone();
                let method_name_str = method_name.to_string();
                let case = if method.sig.inputs.is_empty() {
                    quote! {
                        #method_name_str => #contract_name::#method_name(),
                    }
                } else {
                    quote! {
                        #method_name_str => #contract_name::#method_name(call.try_deserialize_args().unwrap()),
                    }
                };
                Some(case)
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<_>>()
    } else {
        vec![]
    };

    let entrypoint_and_contract = quote! {
        use multivm_sdk::{
            env,
            multivm_primitives::{AccountId, ContractCall},
        };

        multivm_sdk::entrypoint!(entrypoint);

        pub fn entrypoint(call: ContractCall) {
            match call.method.as_str() {
                #(#methods) *

                _ => {
                    panic!("Unknown method name");
                }
            }
        }

        #quote_input
    };

    TokenStream::from(entrypoint_and_contract)
}

#[proc_macro_attribute]
pub fn generate_payload(_: TokenStream, input: TokenStream) -> TokenStream {
    let function = parse_macro_input!(input as ItemFn);

    let fn_name = &function.sig.ident;
    let fn_args = function.sig.inputs;

    let struct_name = {
        let mut name = fn_name.to_string();
        if let Some(first_char) = name.chars().next() {
            name.replace_range(..1, &first_char.to_uppercase().to_string());
        }
        name += "Payload";
        syn::Ident::new(&name, fn_name.span())
    };

    let struct_fields = fn_args.iter().map(|arg| {
        if let syn::FnArg::Typed(pat_type) = arg {
            if let syn::Pat::Ident(ident) = &*pat_type.pat {
                let field_name = &ident.ident;
                let field_type = &pat_type.ty;
                quote! { pub #field_name: #field_type }
            } else {
                panic!("Invalid function argument pattern");
            }
        } else {
            panic!("Invalid function argument");
        }
    });

    let struct_tokens = quote! {
        struct #struct_name {
            #( #struct_fields ),*
        }
    };

    TokenStream::from(struct_tokens)
}
