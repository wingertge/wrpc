use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;

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
    pub fn to_tokens(&self, options: &RpcAttribute) -> proc_macro2::TokenStream {
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

        let sig = quote!(#name(#(#args),*) -> ::wrpc::Result<#return_type>);
        let body = self.client_body(options);

        quote! {
            #sig {
                #body
            }
        }
    }

    pub fn client_body(&self, options: &RpcAttribute) -> proc_macro2::TokenStream {
        let RpcAttribute {
            method,
            path,
            return_override,
        } = options;

        let mut path = path
            .split('/')
            .map(|segment| {
                if let Some(segment) = segment.strip_prefix(':') {
                    format!("{{{segment}}}")
                } else {
                    segment.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("/");

        let query_binding = if let Some((name, _)) = &self.query {
            path += "?{__query}";
            quote!(let __query = ::serde_qs::to_string(#name);)
        } else {
            quote!()
        };

        let path = if self.query.is_some() || self.path.is_some() {
            quote!(&::std::format!(#path))
        } else {
            quote!(#path)
        };

        let body = if let Some(name) = &self.body {
            quote!(.body(::std::string::ToString::to_string(#name)))
        } else if let Some((name, _)) = &self.json {
            quote!(.body(::serde_json::to_string(#name)))
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

        quote! {
            #query_binding
            ::reqwasm::http::Request::#method(#path)
                #body
                .send()
                .await?
                #result_extractor
                .await
        }
    }
}
