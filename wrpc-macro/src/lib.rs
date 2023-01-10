use argument::{Argument, ArgumentType};
use proc_macro::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, Ident, ItemFn, ReturnType, Signature, Type};

extern crate proc_macro;

mod argument;
mod attr;
mod codegen;
#[cfg(test)]
mod test;

/// Generates the client side code to call this API function.
/// Currently only works with `axum` due to the large variance in request
/// extractor syntax between frameworks.
///
/// # Quick Usage
///
/// ```
/// # use axum::{Json, extract::Path};
/// # use wrpc_macro::rpc;
/// # #[derive(serde::Serialize, serde::Deserialize)]
/// # struct User;
///
/// #[rpc(get("/api/user/:id"))]
/// pub async fn get_user(Path(id): Path<u32>) -> Json<User> {
///     // Do things here
///     Json(User)
/// }
/// ```
///
/// This will gate the handler to only exist on non-WASM targets and create a
/// WASM side function somewhat like this:
///
/// ```
/// # #[derive(serde::Serialize, serde::Deserialize)]
/// # struct User;
/// pub async fn call_get_user(id: u32) -> Result<User, reqwasm::Error> {
///     reqwasm::http::Request::get(&format!("/api/user/{id}"))
///         .send()
///         .await?
///         .json()
///         .await
/// }
/// ```
///
/// # Configuration
///
/// * `get(path)` - Specifiy this handler's path relative to the root of your
/// API. Extracted path segments are prefixed with `:`, i.e. `:id`.
/// * `returns(Type)` - Specify an overriding return type for your client side
/// function. This must be either `String` or a deserializable type. It's mostly
/// useful for handlers that return status codes or have an otherwise more
/// complex return type.
///
/// # Requirements
///
/// * Path inputs with multiple segments must be destructured. This is because
/// the macro separates these parameters into separate arguments to the client
/// side function and needs their names.
/// * Text body inputs must be `String`s
/// * All request-derived inputs must be `Json`, `Query`, `Path` or `String`.
/// Any other arguments are assumed to be state derived and skipped.
/// * The return type must be `Json` or `String`/`&str`. `&str` will be turned
/// into `String` on the client side.
/// * The full path to the API handler must be specified. wrpc currently can't
/// have access to your Router, so paths are unknown to the macro.
///
#[proc_macro_attribute]
pub fn rpc(attr: TokenStream, item: TokenStream) -> TokenStream {
    match rpc_impl(attr.into(), item.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn rpc_impl(
    attr: proc_macro2::TokenStream,
    item: proc_macro2::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    let handler: ItemFn = syn::parse2(item)?;
    let vis = &handler.vis;
    let sig: RpcSignature = handler.sig.clone().try_into()?;
    let options = syn::parse2(attr)?;

    let client_fn = sig.to_tokens(&options, vis);

    let tokens_new = quote! {
        #[cfg(any(not(target_arch = "wasm32"), not(client)))]
        #handler

        #client_fn
    };

    Ok(tokens_new)
}

#[derive(Debug)]
struct RpcSignature {
    pub name: Ident,
    pub path: Option<Vec<(Ident, Type)>>,
    pub query: Option<(Ident, Type)>,
    pub body: Option<Ident>,
    pub json: Option<(Ident, Type)>,
    pub return_type: ArgumentType,
}

impl TryFrom<Signature> for RpcSignature {
    type Error = syn::Error;

    fn try_from(value: Signature) -> Result<Self, Self::Error> {
        let args: Vec<Argument> = value
            .inputs
            .into_iter()
            .map(|arg| arg.try_into())
            .collect::<Result<_, _>>()?;
        let ret = {
            let span = value.output.span();
            match value.output {
                ReturnType::Type(_, ty) => Ok(ty),
                ReturnType::Default => {
                    Err(syn::Error::new(span, "Rpc functions must have a return"))
                }
            }
        }?;

        let mut signature = RpcSignature {
            name: value.ident,
            path: None,
            query: None,
            body: None,
            json: None,
            return_type: ret.try_into()?,
        };

        for arg in args {
            match arg {
                Argument::Json { name, inner_type } => {
                    signature.json = Some((name, inner_type));
                }
                Argument::Query { name, inner_type } => {
                    signature.query = Some((name, inner_type));
                }
                Argument::Path { inner_types } => {
                    signature.path = Some(inner_types);
                }
                Argument::Body { name } => {
                    signature.body = Some(name);
                }
                Argument::Ignored => {}
            }
        }

        if signature.body.is_some() && signature.json.is_some() {
            signature.body = None;
        }

        Ok(signature)
    }
}
