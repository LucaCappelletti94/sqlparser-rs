// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Standard trait derives without `#[inline]`, so downstream crates link to
//! one copy instead of each instantiating their own.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{ext::IdentExt, Data, DeriveInput, Fields, GenericParam, Generics, Ident};

struct Arm<'a> {
    path: TokenStream,
    name: String,
    fields: &'a Fields,
}

fn arms(input: &DeriveInput) -> Vec<Arm<'_>> {
    let name = &input.ident;
    match &input.data {
        Data::Struct(s) => vec![Arm {
            path: quote!(#name),
            name: input.ident.unraw().to_string(),
            fields: &s.fields,
        }],
        Data::Enum(e) => e
            .variants
            .iter()
            .map(|v| {
                assert!(
                    v.discriminant.is_none(),
                    "explicit discriminants are not supported"
                );
                let ident = &v.ident;
                Arm {
                    path: quote!(#name::#ident),
                    name: ident.unraw().to_string(),
                    fields: &v.fields,
                }
            })
            .collect(),
        Data::Union(_) => panic!("unions are not supported"),
    }
}

fn bindings(fields: &Fields, prefix: &str) -> (TokenStream, Vec<Ident>) {
    match fields {
        Fields::Named(named) => {
            let names: Vec<_> = named
                .named
                .iter()
                .map(|f| f.ident.as_ref().unwrap())
                .collect();
            let binds: Vec<_> = names
                .iter()
                .map(|n| format_ident!("{}{}", prefix, n.unraw()))
                .collect();
            (quote!({ #(#names: #binds),* }), binds)
        }
        Fields::Unnamed(unnamed) => {
            let binds: Vec<_> = (0..unnamed.unnamed.len())
                .map(|i| format_ident!("{}{}", prefix, i))
                .collect();
            (quote!(( #(#binds),* )), binds)
        }
        Fields::Unit => (quote!(), Vec::new()),
    }
}

fn rebuild(fields: &Fields, values: &[TokenStream]) -> TokenStream {
    match fields {
        Fields::Named(named) => {
            let names = named.named.iter().map(|f| f.ident.as_ref().unwrap());
            quote!({ #(#names: #values),* })
        }
        Fields::Unnamed(_) => quote!(( #(#values),* )),
        Fields::Unit => quote!(),
    }
}

fn bounded(generics: &Generics, bound: TokenStream) -> Generics {
    let mut generics = generics.clone();
    for param in &mut generics.params {
        if let GenericParam::Type(ty) = param {
            ty.bounds.push(syn::parse2(bound.clone()).unwrap());
        }
    }
    generics
}

pub(crate) fn derive_clone(input: DeriveInput) -> TokenStream {
    let arms = arms(&input).into_iter().map(|arm| {
        let (pattern, binds) = bindings(arm.fields, "__self_");
        let values: Vec<_> = binds
            .iter()
            .map(|b| quote!(::core::clone::Clone::clone(#b)))
            .collect();
        let rebuilt = rebuild(arm.fields, &values);
        let path = arm.path;
        quote!(#path #pattern => #path #rebuilt,)
    });
    let name = &input.ident;
    let generics = bounded(&input.generics, quote!(::core::clone::Clone));
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics ::core::clone::Clone for #name #ty_generics #where_clause {
            fn clone(&self) -> Self {
                match self { #(#arms)* }
            }
        }
    }
}

pub(crate) fn derive_debug(input: DeriveInput) -> TokenStream {
    let arms = arms(&input).into_iter().map(|arm| {
        let (pattern, binds) = bindings(arm.fields, "__self_");
        let label = &arm.name;
        let body = match arm.fields {
            Fields::Named(named) => {
                let names = named
                    .named
                    .iter()
                    .map(|f| f.ident.as_ref().unwrap().unraw().to_string());
                quote!(f.debug_struct(#label) #(.field(#names, #binds))* .finish())
            }
            Fields::Unnamed(_) => quote!(f.debug_tuple(#label) #(.field(#binds))* .finish()),
            Fields::Unit => quote!(f.write_str(#label)),
        };
        let path = arm.path;
        quote!(#path #pattern => #body,)
    });
    let name = &input.ident;
    let generics = bounded(&input.generics, quote!(::core::fmt::Debug));
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics ::core::fmt::Debug for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self { #(#arms)* }
            }
        }
    }
}

pub(crate) fn derive_partial_eq(input: DeriveInput) -> TokenStream {
    let arms: Vec<_> = arms(&input)
        .into_iter()
        .map(|arm| {
            let (left, lhs) = bindings(arm.fields, "__self_");
            let (right, rhs) = bindings(arm.fields, "__other_");
            let path = arm.path;
            quote!((#path #left, #path #right) => true #(&& #lhs == #rhs)*,)
        })
        .collect();
    let fallback = (arms.len() > 1).then(|| quote!(_ => false,));
    let name = &input.ident;
    let generics = bounded(&input.generics, quote!(::core::cmp::PartialEq));
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics ::core::cmp::PartialEq for #name #ty_generics #where_clause {
            fn eq(&self, other: &Self) -> bool {
                match (self, other) { #(#arms)* #fallback }
            }
        }
    }
}

pub(crate) fn derive_hash(input: DeriveInput) -> TokenStream {
    let hashes_discriminant = matches!(&input.data, Data::Enum(e) if e.variants.len() > 1);
    let arms = arms(&input).into_iter().map(|arm| {
        let (pattern, binds) = bindings(arm.fields, "__self_");
        let path = arm.path;
        quote!(#path #pattern => { #(::core::hash::Hash::hash(#binds, &mut state);)* })
    });
    let discriminant = hashes_discriminant
        .then(|| quote!(::core::hash::Hash::hash(&::core::mem::discriminant(value), &mut state);));
    let name = &input.ident;
    let generics = bounded(&input.generics, quote!(::core::hash::Hash));
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        const _: () = {
            fn hash_dyn #impl_generics (
                value: &#name #ty_generics,
                mut state: &mut dyn ::core::hash::Hasher,
            ) #where_clause {
                #discriminant
                match value { #(#arms)* }
            }
            impl #impl_generics ::core::hash::Hash for #name #ty_generics #where_clause {
                #[inline]
                fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                    hash_dyn(self, state)
                }
            }
        };
    }
}

fn wildcard(fields: &Fields) -> TokenStream {
    match fields {
        Fields::Named(_) => quote!({ .. }),
        Fields::Unnamed(_) => quote!((..)),
        Fields::Unit => quote!(),
    }
}

fn derive_ordering(
    input: DeriveInput,
    trait_path: TokenStream,
    method: TokenStream,
    output: TokenStream,
    equal: TokenStream,
) -> TokenStream {
    let arms = arms(&input);
    let compare_arms = arms.iter().map(|arm| {
        let (left, lhs) = bindings(arm.fields, "__self_");
        let (right, rhs) = bindings(arm.fields, "__other_");
        let mut pairs = lhs.iter().zip(&rhs).rev();
        let body = match pairs.next() {
            None => equal.clone(),
            Some((l, r)) => pairs.fold(quote!(#trait_path::#method(#l, #r)), |rest, (l, r)| {
                quote!(match #trait_path::#method(#l, #r) { #equal => #rest, cmp => cmp })
            }),
        };
        let path = &arm.path;
        quote!((#path #left, #path #right) => #body,)
    });
    let fallback = (arms.len() > 1).then(|| {
        let index_arms = arms.iter().enumerate().map(|(i, arm)| {
            let path = &arm.path;
            let wildcard = wildcard(arm.fields);
            let i = i as isize;
            quote!(#path #wildcard => #i,)
        });
        quote! {
            _ => {
                let index = |value: &Self| -> isize { match value { #(#index_arms)* } };
                #trait_path::#method(&index(self), &index(other))
            }
        }
    });
    let name = &input.ident;
    let generics = bounded(&input.generics, trait_path.clone());
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics #trait_path for #name #ty_generics #where_clause {
            fn #method(&self, other: &Self) -> #output {
                match (self, other) { #(#compare_arms)* #fallback }
            }
        }
    }
}

pub(crate) fn derive_partial_ord(input: DeriveInput) -> TokenStream {
    derive_ordering(
        input,
        quote!(::core::cmp::PartialOrd),
        quote!(partial_cmp),
        quote!(::core::option::Option<::core::cmp::Ordering>),
        quote!(::core::option::Option::Some(::core::cmp::Ordering::Equal)),
    )
}

pub(crate) fn derive_ord(input: DeriveInput) -> TokenStream {
    derive_ordering(
        input,
        quote!(::core::cmp::Ord),
        quote!(cmp),
        quote!(::core::cmp::Ordering),
        quote!(::core::cmp::Ordering::Equal),
    )
}
