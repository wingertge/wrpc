use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{spanned::Spanned, Visibility};

use crate::{argument::ArgumentType, attr::RpcAttribute, RpcSignature};

impl ToTokens for ArgumentType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ty = match self {
            ArgumentType::Json(inner) => quote!(#inner),
            ArgumentType::Query(inner) => quote!(#inner),
            ArgumentType::Path(inners) => quote!(#(#inners),*),
            ArgumentType::Body => quote!(String),
            ArgumentType::Ignored => quote!(),
        };
        tokens.extend(ty);
    }
}

impl RpcSignature {
    pub fn to_tokens(&self, options: &RpcAttribute, vis: &Visibility) -> proc_macro2::TokenStream {
        let Self {
            name, return_type, ..
        } = self;

        let mut args = Vec::new();
        if let Some(vars) = &self.path {
            let vars = vars.iter().map(|(name, ty)| quote!(#name: #ty));
            args.extend(vars);
        }
        if let Some((name, ty)) = &self.query {
            args.push(quote!(#name: &#ty));
        }
        if let Some(name) = &self.body {
            args.push(quote!(#name: &str));
        }
        if let Some((name, ty)) = &self.json {
            args.push(quote!(#name: &#ty));
        }
        println!("{options:?}");
        let return_type = options.return_override.as_ref().unwrap_or(return_type);

        let name = format_ident!("call_{name}");
        let sig = quote!(#vis async fn #name(#(#args),*) -> ::wrpc::Result<#return_type>);
        let wasm_body = self.wasm_body(options);
        let reqwest_body = self.reqwest_body(options);

        quote! {
            #[cfg(target_arch = "wasm32")]
            #sig {
                #wasm_body
            }

            #[cfg(not(target_arch = "wasm32"))]
            #sig {
                #reqwest_body
            }
        }
    }

    pub fn wasm_body(&self, options: &RpcAttribute) -> proc_macro2::TokenStream {
        let (path, request_signature) = self.request_signature(options);
        let method = &options.method;

        quote! {
            ::reqwasm::http::Request::#method(#path)
                #request_signature
        }
    }

    pub fn reqwest_body(&self, options: &RpcAttribute) -> proc_macro2::TokenStream {
        let (path, request_signature) = self.request_signature(options);
        let method = &options.method;

        quote! {
            let client = ::reqwest::Client::new();
            client.#method(#path)
                #request_signature
        }
    }

    fn request_signature(&self, options: &RpcAttribute) -> (TokenStream, TokenStream) {
        let RpcAttribute {
            path,
            return_override,
            ..
        } = options;

        let mut segments = vec![];
        let mut path = path
            .split('/')
            .map(|segment| {
                if let Some(segment) = segment.strip_prefix(':') {
                    segments.push(format_ident!("{segment}"));
                    "{}".to_string()
                } else {
                    segment.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("/");

        let query_binding = if let Some((name, _)) = &self.query {
            path += "?{}";
            Some(quote!(::serde_qs::to_string(#name).unwrap()))
        } else {
            None
        };

        let path = if !segments.is_empty() || query_binding.is_some() {
            let mut segments = quote!(#(,#segments)*);
            if let Some(query_binding) = query_binding {
                segments.extend(quote!(,#query_binding));
            }
            quote!(&::std::format!(#path #segments))
        } else {
            quote!(#path)
        };

        let body = if let Some(name) = &self.body {
            quote!(.body(::std::string::ToString::to_string(#name)))
        } else if let Some((name, _)) = &self.json {
            quote!(.body(::serde_json::to_string(#name).unwrap()))
        } else {
            quote!()
        };

        let result_extractor = if matches!(self.return_type, ArgumentType::Json(_)) {
            quote_spanned!(self.return_type.span() => .json())
        } else if let Some(return_override) = return_override {
            quote_spanned!(return_override.span() => .json())
        } else {
            quote_spanned!(self.return_type.span() => .text())
        };

        (
            path,
            quote! {
                #body
                .send()
                .await?
                #result_extractor
                .await
            },
        )
    }
}
