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
        AngleBracketedGenericArguments, Expr, ExprLit, FnArg, GenericArgument, Ident, ItemFn, Lit,
        MetaNameValue, PatType, Path, PathArguments, PathSegment, Receiver, ReturnType, Signature,
        Token, Type, TypeArray, TypePath, TypeReference, Visibility,
        parse::{Parse, ParseStream},
        punctuated::Punctuated,
        spanned::Spanned,
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
        let make_args = |arg_count, name| {
            (0..arg_count).map(move |i| Ident::new(&format!("{name}_{i}"), Span::call_site()))
        };
        let required_args = make_args(required, "required").collect::<Vec<_>>();
        let optional_args = make_args(optional, "optional").collect::<Vec<_>>();
        let rest_arg = make_args(rest.into(), "rest").collect::<Vec<_>>();

        quote! {
            #vis struct #struct_ident;

            impl crate::guile::GuileFn for #struct_ident {
                const REQUIRED: ::core::primitive::usize = #required;
                const OPTIONAL: ::core::primitive::usize = #optional;
                const REST: ::core::primitive::bool = #rest;

                const NAME: &::core::ffi::CStr = #guile_ident;
                const DRIVER: crate::guile::sys::scm_t_subr = {
                    assert!(Self::REQUIRED <= ::core::ffi::c_int::MAX as usize, "array lengths must be less than `i32::MAX`");
                    assert!(Self::OPTIONAL <= ::core::ffi::c_int::MAX as usize, "array lengths must be less than `i32::MAX`");

                    extern "C" fn driver(
                        #(#required_args: crate::guile::sys::SCM,)*
                        #(#optional_args: crate::guile::sys::SCM,)*
                        #(#rest_arg: crate::guile::sys::SCM),*
                    ) -> crate::guile::Scm {
                        let mut api = unsafe { crate::guile::Api::new_unchecked() };
                        #fn_ident(
                            &mut api,
                            [#(crate::guile::Scm::new(#required_args)),*],
                            [#({
                                if #optional_args == unsafe { crate::guile::sys::REEXPORTS_SCM_UNDEFINED } {
                                    ::core::option::Option::None
                                } else {
                                    ::core::option::Option::Some(crate::guile::Scm::new(#optional_args))
                                }
                            }),*],
                            #(crate::guile::Scm::new(#rest_arg),)*
                        )
                    }

                    driver as crate::guile::sys::scm_t_subr
                };
            }
        }
    }
}

#[derive(Default)]
struct ConfigBuilder {
    struct_ident: Option<String>,
    guile_ident: Option<String>,
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
            struct_ident: self
                .struct_ident
                .map(|ident| Ident::new(&ident, ident.span()))
                .unwrap_or_else(|| {
                    Ident::new(
                        &RenameRule::PascalCase.apply_to_field(fn_ident.to_string()),
                        guile_ident.span(),
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
                        value.span(),
                        "arguments may only be string literals",
                    ));
                };

                let ident = if path.is_ident("struct_ident") {
                    &mut accum.struct_ident
                } else if path.is_ident("guile_ident") {
                    &mut accum.guile_ident
                } else {
                    return Err(syn::Error::new(
                        path.get_ident().map(|ident| ident.span()).unwrap_or_else(Span::call_site),
                        format!("Unknown argument `{}`. Available arguments are: `struct_ident`, and `guile_ident`.", path.get_ident().map(<_>::to_string).unwrap_or_else(|| "<??>".to_string()))
                    ));
                };
                *ident = Some(value.value());

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
fn is_path<S>(ty: &Type, ident: &S) -> bool
where
    S: AsRef<str> + ?Sized,
{
    match ty {
        Type::Path(TypePath { qself: None, path }) => path.is_ident(ident.as_ref()),
        _ => false,
    }
}
fn is_api(ty: &Type) -> bool {
    is_path(ty, "Api")
}
fn is_scm(ty: &Type) -> bool {
    is_path(ty, "Scm")
}
fn is_ref_mut<F>(ty: &Type, inner: F) -> bool
where
    F: FnOnce(&Type) -> bool,
{
    match ty {
        Type::Reference(TypeReference {
            mutability: Some(_),
            elem,
            ..
        }) => inner(&elem),
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
/// Return the length expression if the type is an array that passes `predicate`
fn is_array<F>(ty: Type, predicate: F) -> Option<Expr>
where
    F: FnOnce(&Type) -> bool,
{
    match ty {
        Type::Array(TypeArray { elem, len, .. }) if predicate(&elem) => Some(len),
        _ => None,
    }
}
fn expr_to_usize(expr: Expr) -> Result<usize, syn::Error> {
    match &expr {
        Expr::Lit(ExprLit {
            lit: Lit::Int(int), ..
        }) => int.base10_parse::<usize>().map(Some),
        _ => Ok(None),
    }
    .and_then(|len| {
        len.ok_or_else(|| syn::Error::new(expr.span(), "expressions must be static integers"))
    })
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
        let args_span = args.span();
        let mut args = args.into_iter().map(get_type);
        args.next()
            .map_or_else(
                || Ok(()),
                |arg| {
                    is_ref_mut(&arg, is_api).then_some(()).ok_or_else(|| {
                        syn::Error::new(arg.span(), "the first argument must be of type `&mut Api`")
                    })
                },
            )
            .and_then(|_| {
                const ERROR: &str = "the second argument must be of type `[Scm; LEN]`";
                args.next().map_or_else(
                    || Err(syn::Error::new(args_span, ERROR)),
                    |ty| {
                        let ty_span = ty.span();
                        is_array(*ty, is_scm).ok_or_else(|| syn::Error::new(ty_span, ERROR))
                    },
                )
            })
            .and_then(expr_to_usize)
            .and_then(|required| {
                const ERROR: &str = "the third argument must be of type `[Option<Scm>; LEN]`";
                args.next()
                    .map_or_else(
                        || Err(syn::Error::new(args_span, ERROR)),
                        |ty| {
                            let ty_span = ty.span();
                            is_array(*ty, is_option_scm)
                                .ok_or_else(|| syn::Error::new(ty_span, ERROR))
                        },
                    )
                    .and_then(expr_to_usize)
                    .map(|optional| (required, optional))
            })
            .and_then(|(required, optional)| {
                args.next()
                    .map(|arg| {
                        if is_scm(&arg) {
                            Ok(true)
                        } else {
                            Err(syn::Error::new(
                                arg.span(),
                                "the optional third argument must be of type `Scm`",
                            ))
                        }
                    })
                    .unwrap_or(Ok(false))
                    .map(|rest| Inputs {
                        required,
                        optional,
                        rest,
                    })
            })
    }
}

fn assert_none<T>(option: Option<T>, token: &str) -> Result<(), syn::Error>
where
    T: Spanned,
{
    match option {
        Some(item) => Err(syn::Error::new(
            item.span(),
            format!("function cannot be {token}"),
        )),
        None => Ok(()),
    }
}

#[proc_macro_attribute]
pub fn guile_fn(config: TokenStream, input: TokenStream) -> TokenStream {
    syn::parse::<ItemFn>(input.clone())
        .and_then(
            |ItemFn {
                 vis,
                 sig:
                     Signature {
                         constness,
                         asyncness,
                         unsafety,
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
                        assert_none(
                            (generics != Default::default()).then_some(generics),
                            "generic",
                        )
                    })
                    .and_then(|_| match output {
                        ReturnType::Type(_, ty) if is_scm(&ty) => Ok(()),
                        _ => Err(syn::Error::new(output.span(), "return type must be `Scm`")),
                    })
                    .and_then(|_| syn::parse::<ConfigBuilder>(config))
                    .and_then(|builder| Inputs::try_from(inputs).map(|inputs| (builder, inputs)))
                    .and_then(|(builder, inputs)| {
                        let fn_ident_span = fn_ident.span();
                        builder.build(vis, fn_ident, inputs).map_err(|error| {
                            syn::Error::new(
                                fn_ident_span,
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
