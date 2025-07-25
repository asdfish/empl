// empl - Extensible Music PLayer
// Copyright (C) 2025  Andrew Chi

// This file is part of empl.

// empl is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// empl is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with empl.  If not, see <http://www.gnu.org/licenses/>.

use {
    ident_case::RenameRule,
    proc_macro::TokenStream,
    proc_macro2::{Span, TokenStream as TokenStream2},
    quote::quote,
    std::ffi::{CString, NulError},
    syn::{
        Abi, AngleBracketedGenericArguments, Expr, ExprLit, FnArg, GenericArgument, Ident, ItemFn,
        Lit, MetaNameValue, PatType, Path, PathArguments, PathSegment, Receiver, ReturnType,
        Signature, Token, Type, TypePath, Visibility,
        parse::{Parse, ParseStream},
        punctuated::Punctuated,
    },
};

struct Config {
    struct_ident: Ident,
    guile_ident: CString,
    fn_ident: Ident,
    inputs: Inputs,
    vis: Visibility,
}
impl From<Config> for TokenStream2 {
    fn from(
        Config {
            struct_ident,
            guile_ident,
            fn_ident,
            inputs:
                Inputs {
                    required,
                    optional,
                    rest,
                },
            vis,
        }: Config,
    ) -> TokenStream2 {
        quote! {
            #vis struct #struct_ident;

            impl crate::guile::GuileFn for #struct_ident {
                const REQUIRED: ::core::primitive::usize = #required;
                const OPTIONAL: ::core::primitive::usize = #optional;
                const REST: ::core::primitive::bool = #rest;

                const NAME: &::core::ffi::CStr = #guile_ident;
                const ADDR: crate::guile::sys::scm_t_subr = { #fn_ident as crate::guile::sys::scm_t_subr };
            }
        }
    }
}

#[derive(Default)]
struct ConfigBuilder {
    struct_ident: Option<Ident>,
    guile_ident: Option<Ident>,
}
impl ConfigBuilder {
    pub fn build(
        self,
        vis: Visibility,
        fn_ident: Ident,
        inputs: Inputs,
    ) -> Result<Config, NulError> {
        CString::new(
            self.guile_ident
                .map(|ident| ident.to_string())
                .unwrap_or_else(|| RenameRule::KebabCase.apply_to_field(fn_ident.to_string())),
        )
        .map(|guile_ident| Config {
            struct_ident: self.struct_ident.unwrap_or_else(|| {
                Ident::new(
                    &RenameRule::PascalCase.apply_to_field(fn_ident.to_string()),
                    Span::call_site(),
                )
            }),
            guile_ident,
            inputs,
            fn_ident,
            vis,
        })
    }
}
impl Parse for ConfigBuilder {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        Punctuated::<MetaNameValue, Token![,]>::parse_terminated(input).and_then(|items| {
            items.into_iter().try_fold(Self::default(), |mut accum, value| {
                let MetaNameValue {
                    path,
                    value:
                        Expr::Lit(ExprLit {
                            lit: Lit::Str(value),
                            ..
                        }),
                    ..
                } = value
                else {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        "arguments may only be string literals",
                    ));
                };

                let ident = if path.is_ident("struct_ident") {
                    &mut accum.struct_ident
                } else if path.is_ident("guile_ident") {
                    &mut accum.guile_ident
                } else {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        format!("Unknown argument `{}`. Available arguments are: `struct_ident`, and `guile_ident`.", path.get_ident().map(<_>::to_string).unwrap_or_else(|| "<??>".to_string()))
                    ));
                };
                *ident = Some(Ident::new(&value.value(), Span::call_site()));

                Ok(accum)
            })
        })
    }
}

fn get_type(arg: FnArg) -> Box<Type> {
    match arg {
        FnArg::Receiver(Receiver { ty, .. }) | FnArg::Typed(PatType { ty, .. }) => ty,
    }
}
fn is_scm(ty: &Type) -> bool {
    match ty {
        Type::Path(TypePath { qself: None, path }) => path.is_ident("Scm"),
        _ => false,
    }
}
fn is_option_scm(ty: &Type) -> bool {
    match ty {
        Type::Path(TypePath {
            qself: None,
            path: Path { segments, .. },
        }) => segments
            .last()
            .map(|PathSegment { ident, arguments }| {
                Ident::new("Option", Span::call_site()).eq(ident)
                    && matches!(
                        arguments,
                        PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                            args,
                            ..
                        }) if args.first().map(|arg| matches!(arg, GenericArgument::Type(ty) if is_scm(ty))).unwrap_or_default()
                    )
            })
            .unwrap_or_default(),
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Inputs {
    required: usize,
    optional: usize,
    rest: bool,
}
impl TryFrom<Punctuated<FnArg, Token![,]>> for Inputs {
    type Error = syn::Error;

    fn try_from(args: Punctuated<FnArg, Token![,]>) -> Result<Self, Self::Error> {
        let mut iter = args.into_iter().map(get_type);

        let output = Self {
            required: iter.by_ref().take_while(|ty| is_scm(ty)).count(),
            optional: iter.by_ref().take_while(|ty| is_option_scm(ty)).count(),
            rest: iter.next().filter(|ty| is_scm(ty)).is_some(),
        };
        if let Some(arg) = iter.next() {
            Err(syn::Error::new(
                Span::call_site(),
                format!("Unexpected type `{arg:?}`. Allowed types are: `Scm` and `Option<Scm>.`"),
            ))
        } else {
            Ok(output)
        }
    }
}

fn assert_none<T>(option: Option<T>, token: &str) -> Result<(), syn::Error> {
    match option {
        Some(_) => Err(syn::Error::new(
            Span::call_site(),
            format!("function cannot be {token}"),
        )),
        None => Ok(()),
    }
}

#[proc_macro_attribute]
pub fn raw_subr(config: TokenStream, input: TokenStream) -> TokenStream {
    syn::parse::<ItemFn>(input.clone())
        .and_then(
            |ItemFn {
                 vis,
                 sig:
                     Signature {
                         constness,
                         asyncness,
                         unsafety,
                         abi,
                         variadic,
                         generics,
                         ident: fn_ident,
                         inputs,
                         output,
                         ..
                     },
                 ..
             }| {
                assert_none(constness, "const")
                    .and_then(|_| assert_none(asyncness, "async"))
                    .and_then(|_| assert_none(unsafety, "unsafe"))
                    .and_then(|_| assert_none(variadic, "variadic"))
                    .and_then(|_| {
                        assert_none((generics != Default::default()).then_some(()), "generic")
                    })
                    .and_then(|_| match output {
                        ReturnType::Type(_, ty) if is_scm(&ty) => Ok(()),
                        _ => Err(syn::Error::new(
                            Span::call_site(),
                            "return type must be `Scm`",
                        )),
                    })
                    .and_then(|_| match abi {
                        Some(Abi {
                            name: Some(name), ..
                        }) if name.value() == "C" => Ok(()),
                        _ => Err(syn::Error::new(
                            Span::call_site(),
                            "abi must be `extern \"C\"`",
                        )),
                    })
                    .and_then(|_| syn::parse::<ConfigBuilder>(config))
                    .and_then(|builder| Inputs::try_from(inputs).map(|inputs| (builder, inputs)))
                    .and_then(|(builder, inputs)| {
                        builder.build(vis, fn_ident, inputs).map_err(|error| {
                            syn::Error::new(
                                Span::call_site(),
                                format!("identifiers cannot have nul bytes: {error}"),
                            )
                        })
                    })
                    .map(TokenStream2::from)
                    .map(|mut tokens| {
                        tokens.extend([input].map(TokenStream2::from));
                        tokens
                    })
            },
        )
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
