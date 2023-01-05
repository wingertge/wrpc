use syn::{parenthesized, parse::Parse, Ident, LitStr, Token, Type};

use crate::argument::ArgumentType;

#[derive(Debug)]
pub struct RpcAttribute {
    pub method: Ident,
    pub path: String,
    pub return_override: Option<ArgumentType>,
}

impl Parse for RpcAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let options = input.parse_terminated::<_, Token![,]>(AttributeOption::parse)?;
        let (method, path, return_override) = options
            .into_iter()
            .map(|option| match option {
                AttributeOption::Method(method, path) => (Some(method), Some(path), None),
                AttributeOption::ReturnOverride(ty) => (None, None, Some(ArgumentType::Json(ty))),
            })
            .fold((None, None, None), |(a1, b1, c1), (a2, b2, c2)| {
                (a1.or(a2), b1.or(b2), c1.or(c2))
            });

        Ok(RpcAttribute {
            method: method.ok_or_else(|| input.error("Missing method"))?,
            path: path.unwrap().value(),
            return_override,
        })
    }
}

enum AttributeOption {
    Method(Ident, LitStr),
    ReturnOverride(Type),
}

impl Parse for AttributeOption {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        const METHODS: &[&str] = &["get", "post", "put", "delete", "patch"];

        let name: Ident = input.parse()?;
        let content;
        parenthesized!(content in input);

        if METHODS.iter().any(|&method| name == method) {
            Ok(AttributeOption::Method(name, content.parse()?))
        } else if name == "returns" {
            Ok(AttributeOption::ReturnOverride(content.parse()?))
        } else {
            Err(syn::Error::new(name.span(), "Unexpected option"))
        }
    }
}
