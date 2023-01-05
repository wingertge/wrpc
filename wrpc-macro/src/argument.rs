use syn::{
    spanned::Spanned, AngleBracketedGenericArguments, FnArg, GenericArgument, Ident, Pat, Type,
    TypeReference,
};

pub enum Argument {
    Json { name: Ident, inner_type: Type },
    Query { name: Ident, inner_type: Type },
    Path { inner_types: Vec<(Ident, Type)> },
    Body { name: Ident },
    Ignored,
}

impl TryFrom<FnArg> for Argument {
    type Error = syn::Error;

    fn try_from(value: FnArg) -> Result<Self, Self::Error> {
        let value = match value {
            FnArg::Receiver(_) => {
                return Ok(Self::Ignored);
            }
            FnArg::Typed(typed) => typed,
        };
        let name: ArgumentName = value.pat.try_into()?;
        let ty: ArgumentType = value.ty.try_into()?;

        Ok(match ty {
            ArgumentType::Json(inner) => Self::Json {
                name: name.single()?,
                inner_type: inner,
            },
            ArgumentType::Query(inner) => Self::Query {
                name: name.single()?,
                inner_type: inner,
            },
            ArgumentType::Path(types) => {
                let names = name.multiple();
                if names.len() == types.len() {
                    let types = names.into_iter().zip(types.into_iter()).collect();
                    Self::Path { inner_types: types }
                } else {
                    return Err(syn::Error::new(
                        types[0].span(),
                        "Path tuples must be destructured",
                    ));
                }
            }
            ArgumentType::Body => Self::Body {
                name: name.single()?,
            },
            ArgumentType::Ignored => Self::Ignored,
        })
    }
}

pub enum ArgumentName {
    Single(Ident),
    Multiple(Vec<Ident>),
}

impl TryFrom<Box<Pat>> for ArgumentName {
    type Error = syn::Error;

    fn try_from(pat: Box<Pat>) -> Result<Self, Self::Error> {
        match *pat {
            Pat::Ident(ident) => Ok(Self::Single(ident.ident)),
            Pat::TupleStruct(tuple) => {
                let elems = if let Some(Pat::Tuple(tuple)) = tuple.pat.elems.first() {
                    &tuple.elems
                } else {
                    &tuple.pat.elems
                };
                if elems.len() == 1 {
                    let name = match elems.first().unwrap() {
                        Pat::Ident(ident) => Ok(ident.ident.clone()),
                        elem => Err(syn::Error::new(
                            elem.span(),
                            "Expected tuple struct pattern to contain only idents",
                        )),
                    }?;
                    Ok(Self::Single(name))
                } else {
                    let names = elems
                        .into_iter()
                        .map(|pat| match pat {
                            Pat::Ident(ident) => Ok(ident.ident.clone()),
                            pat => Err(syn::Error::new(
                                pat.span(),
                                "Expected tuple struct pattern to contain only idents",
                            )),
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(Self::Multiple(names))
                }
            }
            pat => Err(syn::Error::new(
                pat.span(),
                "Expected plain ident or tuple struct for argument name",
            )),
        }
    }
}

impl ArgumentName {
    pub fn single(self) -> Result<Ident, syn::Error> {
        match self {
            ArgumentName::Single(ident) => Ok(ident),
            ArgumentName::Multiple(idents) => Err(syn::Error::new(
                idents[0].span(),
                "Expected single name, found destructured tuple",
            )),
        }
    }

    pub fn multiple(self) -> Vec<Ident> {
        match self {
            ArgumentName::Single(ident) => vec![ident],
            ArgumentName::Multiple(idents) => idents,
        }
    }
}

#[derive(Debug)]
pub enum ArgumentType {
    Json(Type),
    Query(Type),
    Path(Vec<Type>),
    Body,
    Ignored,
}

impl TryFrom<Box<Type>> for ArgumentType {
    type Error = syn::Error;

    fn try_from(value: Box<Type>) -> syn::Result<Self> {
        let ty = match *value {
            Type::Path(path) => Ok(path.path),
            Type::Reference(TypeReference { elem, .. }) => match *elem {
                Type::Path(path) => Ok(path.path),
                value => Err(syn::Error::new(value.span(), "Argument type must be path")),
            },
            Type::ImplTrait(_) => return Ok(Self::Ignored),
            value => Err(syn::Error::new(value.span(), "Argument type must be path")),
        }?;

        let last = ty.segments.into_iter().last().unwrap();
        let arg = match last.arguments {
            syn::PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) => {
                match args.into_iter().next() {
                    Some(GenericArgument::Type(ty)) => Some(ty),
                    _ => None,
                }
            }
            _ => None,
        };

        #[allow(clippy::unnecessary_unwrap)] // The if let alternative is unstable
        if last.ident == "Json" && arg.is_some() {
            Ok(ArgumentType::Json(arg.unwrap()))
        } else if last.ident == "Query" && arg.is_some() {
            Ok(ArgumentType::Query(arg.unwrap()))
        } else if last.ident == "Path" && arg.is_some() {
            let inner_types = match arg.unwrap() {
                Type::Path(path) => Ok(vec![Type::Path(path)]),
                Type::Tuple(tuple) => Ok(tuple.elems.into_iter().collect()),
                arg => Err(syn::Error::new(
                    arg.span(),
                    "Path arguments must be tuples or plain types",
                )),
            }?;
            Ok(ArgumentType::Path(inner_types))
        } else if last.ident == "String" || last.ident == "str" {
            Ok(ArgumentType::Body)
        } else {
            Ok(ArgumentType::Ignored)
        }
    }
}
