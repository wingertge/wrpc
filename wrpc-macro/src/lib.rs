use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::Parse, punctuated::Punctuated, spanned::Spanned, token::Comma,
    AngleBracketedGenericArguments, AttributeArgs, FnArg, GenericArgument, Ident, ItemFn, Lit,
    Meta, MetaList, NestedMeta, Pat, PatTuple, PatTupleStruct, PatType, Path, PathArguments,
    ReturnType, Signature, Token, Type, TypePath, TypeReference, TypeTuple,
};

extern crate proc_macro;

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
/// # struct User;
///
/// #[rpc(get("/api/user/:id"))]
/// pub async fn handler(Path(id): Path<u32>) -> Json<User> {
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
/// pub async fn handler(id: u32) -> Result<User, reqwasm::Error> {
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
    rpc_impl(attr.into(), item.into()).into()
}

fn rpc_impl(
    attr: proc_macro2::TokenStream,
    item: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    //let args = parse_macro_input!(attr as AttributeArgs);
    //let handler = parse_macro_input!(item as ItemFn);
    let AttrArgs(args) = syn::parse2(attr).expect("Failed to parse");
    let handler: ItemFn = syn::parse2(item).expect("Failed to parse");

    let (method, path) = method_and_path(&args);
    let type_override = type_override(&args).map(|path| Type::Path(TypePath { qself: None, path }));
    let ItemFn {
        vis,
        sig:
            Signature {
                ident,
                inputs,
                output,
                ..
            },
        ..
    } = &handler;
    let args = transform_args(inputs);
    let client_signature = args.transformed_signature;
    let return_type = type_override.clone().unwrap_or_else(|| match output {
        ReturnType::Default => panic!("Handlers must specify a return type"),
        ReturnType::Type(_, t) => (**t).to_owned(),
    });
    let return_type = if let Type::Reference(TypeReference { elem, .. }) = &return_type {
        if let Type::Path(TypePath { path, .. }) = &**elem {
            if path.segments.last().unwrap().ident == "str" {
                Type::Path(TypePath {
                    path: Ident::new("String", Span::call_site()).into(),
                    qself: None,
                })
            } else {
                Type::Path(TypePath {
                    qself: None,
                    path: path.to_owned(),
                })
            }
        } else {
            return_type
        }
    } else {
        return_type
    };
    let json_inner_type = extract_json_inner_type(&return_type);
    let result_extractor = if json_inner_type.is_some() || type_override.is_some() {
        Ident::new("json", return_type.span())
    } else {
        Ident::new("text", return_type.span())
    };
    let return_type = json_inner_type.unwrap_or(return_type);
    let mut url_needs_formatting = false;
    let path = if !args.path_segments.is_empty() {
        url_needs_formatting = true;
        path.split('/')
            .map(|segment| {
                let mut segment = segment.to_string();
                if segment.starts_with(':') {
                    segment = segment.replace(':', "{");
                    segment += "}";
                    segment
                } else {
                    segment
                }
            })
            .collect::<Vec<String>>()
            .join("/")
    } else {
        path
    };
    let path = if let Some(query) = &args.query {
        url_needs_formatting = true;
        let pat = &query.pat;
        format!("{path}?{{{}}}", quote!(#pat))
    } else {
        path
    };
    let query_binding = if let Some(query) = args.query {
        let pat = query.pat;
        quote!(let #pat = ::serde_qs::to_string(&#pat);)
    } else {
        quote!()
    };
    let body = if let Some(json) = args.json_arg {
        let name = json.pat;
        quote!(.body(::serde_json::to_string(&#name)))
    } else if let Some(text_arg) = args.text_arg {
        let name = text_arg.pat;
        quote!(.body(#name))
    } else {
        quote!()
    };
    let path = if url_needs_formatting {
        quote!(&::std::format!(#path))
    } else {
        quote!(#path)
    };

    quote! {
        #[cfg(not(target_arch = "wasm32"))]
        #handler

        #[cfg(target_arch = "wasm32")]
        #vis async fn #ident(#(#client_signature),*) -> Result<#return_type, ::reqwasm::Error> {
            #query_binding
            ::reqwasm::http::Request::#method(#path)
                #body
                .send()
                .await?
                .#result_extractor()
                .await
        }
    }
}

fn method_and_path(args: &[NestedMeta]) -> (Ident, String) {
    const METHODS: &[&str] = &["get", "post", "put", "delete", "patch"];

    args.iter()
        .find_map(|meta| match meta {
            NestedMeta::Meta(Meta::List(MetaList {
                path: Path { segments, .. },
                nested,
                ..
            })) if segments.len() == 1
                && METHODS.contains(&segments[0].ident.to_string().as_str())
                && nested.len() == 1 =>
            {
                Some(match &nested[0] {
                    NestedMeta::Lit(Lit::Str(path)) => (segments[0].ident.clone(), path.value()),
                    _ => panic!("Invalid api path"),
                })
            }
            _ => None,
        })
        .expect("Missing method and path from rpc macro")
}

fn type_override(args: &[NestedMeta]) -> Option<Path> {
    args.iter().find_map(|meta| match meta {
        NestedMeta::Meta(Meta::List(MetaList {
            path: Path { segments, .. },
            nested,
            ..
        })) if segments.len() == 1 && segments[0].ident == "returns" && nested.len() == 1 => {
            Some(match &nested[0] {
                NestedMeta::Meta(Meta::Path(path)) => path.to_owned(),
                _ => panic!("Invalid returns clause"),
            })
        }
        _ => None,
    })
}

fn extract_json_inner_type(t: &Type) -> Option<Type> {
    extract_inner_type(t, "Json")
}

fn extract_query_inner_type(t: &Type) -> Option<Type> {
    extract_inner_type(t, "Query")
}

enum PathSegmentType {
    Tuple(TypeTuple),
    Other(Type),
}

fn extract_path_inner_type(t: &Type) -> Option<PathSegmentType> {
    extract_inner_type(t, "Path").map(|t| match t {
        Type::Tuple(tuple) => PathSegmentType::Tuple(tuple),
        _ => PathSegmentType::Other(t),
    })
}

fn extract_inner_type(t: &Type, type_name: &str) -> Option<Type> {
    match t {
        Type::Path(TypePath { path, .. }) => {
            let last = path.segments.last().unwrap();
            if last.ident == type_name {
                match &last.arguments {
                    PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                        args, ..
                    }) if args.len() == 1 => {
                        if let GenericArgument::Type(t) = &args[0] {
                            Some(t.to_owned())
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

struct TransformedArgs {
    transformed_signature: Vec<PatType>,
    query: Option<PatType>,
    json_arg: Option<PatType>,
    path_segments: Vec<PatType>,
    text_arg: Option<PatType>,
}

fn transform_args(args: &Punctuated<FnArg, Comma>) -> TransformedArgs {
    let mut transformed_signature = Vec::with_capacity(args.len());
    let mut query = None;
    let mut json_arg = None;
    let mut path_segments = Vec::new();
    let mut text_arg = None;

    for arg in args.into_iter().filter_map(|arg| match arg {
        FnArg::Typed(pat) => Some(pat),
        _ => None,
    }) {
        let mut arg = arg.clone();

        if let Some(ty) = extract_path_inner_type(&arg.ty) {
            match ty {
                PathSegmentType::Tuple(tuple) => match &*arg.pat {
                    Pat::TupleStruct(PatTupleStruct {
                        pat: PatTuple { elems, .. },
                        ..
                    }) => {
                        let elems: Vec<&Pat> = match elems.first().unwrap() {
                            Pat::Tuple(PatTuple { elems, .. }) => elems.into_iter().collect(),
                            _ => panic!("Path segments must be destructured"),
                        };
                        for (id, elem) in elems.into_iter().enumerate() {
                            let mut arg = arg.clone();
                            arg.pat = Box::new(elem.to_owned());
                            arg.ty = Box::new(tuple.elems[id].to_owned());
                            transformed_signature.push(arg.clone());
                            path_segments.push(arg.clone());
                        }
                    }
                    _ => panic!("Path segments must be destructured"),
                },
                PathSegmentType::Other(ty) => {
                    match &*arg.pat {
                        Pat::TupleStruct(PatTupleStruct {
                            pat: PatTuple { elems, .. },
                            ..
                        }) if elems.len() == 1 => {
                            arg.pat = Box::new(elems.first().unwrap().to_owned());
                        }
                        _ => {}
                    }
                    arg.ty = Box::new(ty);
                    transformed_signature.push(arg.clone());
                    path_segments.push(arg);
                }
            }
            continue;
        }

        match &*arg.pat {
            Pat::TupleStruct(PatTupleStruct {
                pat: PatTuple { elems, .. },
                ..
            }) if elems.len() == 1 => {
                arg.pat = Box::new(elems.first().unwrap().to_owned());
            }
            _ => {}
        }
        if let Some(json_inner) = extract_json_inner_type(&arg.ty) {
            arg.ty = Box::new(json_inner);
            json_arg = Some(arg.clone());
            transformed_signature.push(arg);
        } else if let Some(query_inner) = extract_query_inner_type(&arg.ty) {
            arg.ty = Box::new(query_inner);
            query = Some(arg.clone());
            transformed_signature.push(arg);
        } else if let Type::Path(TypePath {
            path: Path { segments, .. },
            ..
        }) = &*arg.ty
        {
            if segments.last().unwrap().ident == "String" {
                text_arg = Some(arg.clone());
                transformed_signature.push(arg);
            } else {
                println!("Skipped arg {arg:#?} because it didn't fit any extractor pattern");
            }
        } else {
            println!("Skipped arg {arg:#?} because it didn't fit any extractor pattern");
        }
    }

    TransformedArgs {
        transformed_signature,
        query,
        json_arg,
        path_segments,
        text_arg,
    }
}

struct AttrArgs(AttributeArgs);

impl Parse for AttrArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut metas = Vec::new();

        loop {
            if input.is_empty() {
                break;
            }
            let value = input.parse()?;
            metas.push(value);
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(AttrArgs(metas))
    }
}
