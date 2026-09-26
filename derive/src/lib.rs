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

//! Procedural macros for sqlparser.
//!
//! This crate provides:
//! - [`Visit`] and [`VisitMut`] derive macros for AST traversal.
//! - `Shared*` versions of the standard derives, which leave out `#[inline]`.
//! - [`derive_dialect!`] macro for creating custom SQL dialects.

use quote::quote;
use syn::parse_macro_input;

mod dialect;
mod shared;
mod visit;

/// `Clone` without `#[inline]`.
#[proc_macro_derive(SharedClone)]
pub fn derive_shared_clone(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    shared::derive_clone(parse_macro_input!(input as syn::DeriveInput)).into()
}

/// `Debug` without `#[inline]`.
#[proc_macro_derive(SharedDebug)]
pub fn derive_shared_debug(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    shared::derive_debug(parse_macro_input!(input as syn::DeriveInput)).into()
}

/// `PartialEq` without `#[inline]`.
#[proc_macro_derive(SharedPartialEq)]
pub fn derive_shared_partial_eq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    shared::derive_partial_eq(parse_macro_input!(input as syn::DeriveInput)).into()
}

/// `PartialOrd` without `#[inline]`.
#[proc_macro_derive(SharedPartialOrd)]
pub fn derive_shared_partial_ord(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    shared::derive_partial_ord(parse_macro_input!(input as syn::DeriveInput)).into()
}

/// `Ord` without `#[inline]`.
#[proc_macro_derive(SharedOrd)]
pub fn derive_shared_ord(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    shared::derive_ord(parse_macro_input!(input as syn::DeriveInput)).into()
}

/// `Hash` that feeds every hasher through one `&mut dyn Hasher` instantiation.
#[proc_macro_derive(SharedHash)]
pub fn derive_shared_hash(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    shared::derive_hash(parse_macro_input!(input as syn::DeriveInput)).into()
}

/// Implementation of `#[derive(VisitMut)]`
#[proc_macro_derive(VisitMut, attributes(visit))]
pub fn derive_visit_mut(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    visit::derive_visit(
        input,
        &visit::VisitType {
            visit_trait: quote!(VisitMut),
            visitor_trait: quote!(VisitorMut),
            modifier: Some(quote!(mut)),
        },
    )
}

/// Implementation of `#[derive(Visit)]`
#[proc_macro_derive(Visit, attributes(visit))]
pub fn derive_visit_immutable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    visit::derive_visit(
        input,
        &visit::VisitType {
            visit_trait: quote!(Visit),
            visitor_trait: quote!(Visitor),
            modifier: None,
        },
    )
}

/// Procedural macro for deriving new SQL dialects.
#[proc_macro]
pub fn derive_dialect(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as dialect::DeriveDialectInput);
    dialect::derive_dialect(input)
}
