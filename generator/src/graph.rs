// pest-typed. A statically typed version of pest.
// Copyright (c) 2023 黄博奕
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

use crate::config::Config;
use crate::docs::DocComment;

use super::types::{box_type, option_type, result_type, vec_type};
use pest::unicode::unicode_property_names;
use pest_meta::{
    ast::RuleType,
    optimizer::{OptimizedExpr, OptimizedRule},
};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use std::collections::{btree_map, BTreeMap, BTreeSet};
use syn::Index;

pub fn pest() -> TokenStream {
    quote! {::pest_typed}
}

pub fn pest_typed() -> TokenStream {
    quote! {::pest_typed}
}

fn pest_unicode() -> TokenStream {
    quote! {::pest_typed::unicode}
}

fn ident(s: &str) -> Ident {
    format_ident!("r#{}", s)
}

fn rule_wrappers() -> TokenStream {
    quote! {
        rule_wrappers
    }
}

fn constant_wrappers() -> TokenStream {
    quote! {
        constant_wrappers
    }
}

fn unicode_mod() -> TokenStream {
    quote! {
        unicode
    }
}

fn generics() -> TokenStream {
    quote! {
        generics
    }
}

fn pairs() -> TokenStream {
    quote! {pairs}
}

fn ignore(root: &TokenStream) -> TokenStream {
    let generics = generics();
    quote! {
        #root::#generics::Ignored::<'i>
    }
}

#[derive(Clone)]
enum Edge {
    // Type remained.
    Content,
    ContentI(usize),
    // Type wrapped by Option.
    GetFirst,
    GetSecond,
    OptionalContent,
    // Type wrapped by Option.
    Contents,
}
#[derive(Clone)]
enum Node {
    /// - Type: `&#ident`
    /// - Path: `.content.deref()`
    Rule(TokenStream),
    /// - Type: `&#ident`
    /// - Path: ``
    #[cfg(feature = "grammar-extras")]
    Tag(TokenStream),
    // Type remained.
    /// - Type: `#inner`
    /// - Path: `.content`
    Content(Box<Node>),
    /// - Type: `#inner`
    /// - Path: `.content.#index`
    ContentI(usize, Box<Node>),
    // Type wrapped by Option.
    /// - Type: `#opt::<#inner>`
    /// - Path: `.get_first().as_ref().and_then(|e|Some(e #inner)) #flat`
    GetFirst(bool, Box<Node>),
    /// - Type: `#opt::<#inner>`
    /// - Path: `.get_second().as_ref().and_then(|e|Some(e #inner)) #flat`
    GetSecond(bool, Box<Node>),
    /// - Type: `#opt::<#inner>`
    /// - Path: `.content.as_ref().and_then(|e|Some(e #inner)) #flat`
    OptionalContent(bool, Box<Node>),
    // Type wrapped by Vec.
    /// - Type: `#vec::<#inner>`
    /// - Path: `.content.iter().map(|e|e #inner).collect::<#vec<_>>()`
    Contents(Box<Node>),
    // Type wrapped by tuple.
    /// - Type: `(#(#inner),*)`
    /// - Path: `(#(#inner),*)`
    Tuple(Vec<Node>),
}

impl Node {
    fn from_rule(value: TokenStream) -> Self {
        Self::Rule(value)
    }
    #[cfg(feature = "grammar-extras")]
    fn from_tag(value: TokenStream) -> Self {
        Self::Tag(value)
    }
    fn flattenable(&self) -> bool {
        match self {
            Node::Rule(_) => false,
            #[cfg(feature = "grammar-extras")]
            Node::Tag(_) => false,
            Node::Content(inner) | Node::ContentI(_, inner) => inner.flattenable(),
            Node::GetFirst(false, _)
            | Node::GetSecond(false, _)
            | Node::OptionalContent(false, _) => true,
            Node::GetFirst(true, inner)
            | Node::GetSecond(true, inner)
            | Node::OptionalContent(true, inner) => inner.flattenable(),
            Node::Contents(_) | Node::Tuple(_) => false,
        }
    }
    pub fn wrap(self, edge: Edge) -> Self {
        match edge {
            Edge::Content => Self::Content(Box::new(self)),
            Edge::ContentI(i) => Self::ContentI(i, Box::new(self)),
            Edge::GetFirst => Self::GetFirst(self.flattenable(), Box::new(self)),
            Edge::GetSecond => Self::GetSecond(self.flattenable(), Box::new(self)),
            Edge::OptionalContent => Self::OptionalContent(self.flattenable(), Box::new(self)),
            Edge::Contents => Self::Contents(Box::new(self)),
        }
    }
    pub fn merge(self, other: Self) -> Self {
        match self {
            Node::Tuple(vec) => match other {
                Node::Tuple(mut v) => {
                    let mut vec = vec;
                    vec.append(&mut v);
                    Node::Tuple(vec)
                }
                _ => {
                    let mut vec = vec;
                    vec.push(other);
                    Node::Tuple(vec)
                }
            },
            _ => match other {
                Node::Tuple(mut v) => {
                    let mut vec = vec![self];
                    vec.append(&mut v);
                    Node::Tuple(vec)
                }
                _ => Node::Tuple(vec![self, other]),
            },
        }
    }
    pub fn expand(&self, root: &TokenStream) -> (TokenStream, TokenStream) {
        let flat = |flatten: &bool| {
            if *flatten {
                quote! {.flatten()}
            } else {
                quote! {}
            }
        };
        let opt = |flatten: &bool, inner: TokenStream| {
            let opt = option_type();
            if *flatten {
                quote! {#inner}
            } else {
                quote! {#opt::<#inner>}
            }
        };
        let vec = vec_type();
        match self {
            Node::Rule(t) => (
                quote! {{let res = res.content.deref(); res}},
                quote! {&'s #root::#t::<'i>},
            ),
            #[cfg(feature = "grammar-extras")]
            Node::Tag(t) => (quote! {res}, quote! {&'s #root::#t::<'i>}),
            Node::Content(inner) => {
                let (pa, ty) = inner.expand(root);
                (quote! {{let res = &res.content; #pa}}, quote! {#ty})
            }
            Node::ContentI(i, inner) => {
                let (pa, ty) = inner.expand(root);
                let i = Index::from(*i);
                (quote! {{let res = &res.content.#i; #pa}}, quote! {#ty})
            }
            Node::OptionalContent(flatten, inner) => {
                let (pa, ty) = inner.expand(root);
                let flat = flat(flatten);
                (
                    quote! {{let res = res.content.as_ref().and_then(|res| Some(#pa)) #flat; res}},
                    opt(flatten, ty),
                )
            }
            Node::GetFirst(flatten, inner) => {
                let (pa, ty) = inner.expand(root);
                let flat = flat(flatten);
                (
                    quote! {{let res = res.get_first().as_ref().and_then(|res| Some(#pa)) #flat; res}},
                    opt(flatten, ty),
                )
            }
            Node::GetSecond(flatten, inner) => {
                let (pa, ty) = inner.expand(root);
                let flat = flat(flatten);
                (
                    quote! {{let res = res.get_second().as_ref().and_then(|res| Some(#pa)) #flat; res}},
                    opt(flatten, ty),
                )
            }
            Node::Contents(inner) => {
                let (pa, ty) = inner.expand(root);
                (
                    quote! {{let res = res.content.iter().map(|res| #pa).collect::<#vec<_>>(); res}},
                    quote! {#vec::<#ty>},
                )
            }
            Node::Tuple(tuple) => {
                let (pa, ty): (Vec<_>, Vec<_>) = tuple.iter().map(|e| e.expand(root)).unzip();
                (quote! {{let res = (#(#pa),*); res}}, quote! {(#(#ty),*)})
            }
        }
    }
}

/// `'g` stands for the lifetime of rules.
struct Accesser<'g> {
    /// name -> (path, type)
    accessers: BTreeMap<&'g str, Node>,
}
impl<'g> Accesser<'g> {
    pub fn new() -> Self {
        Self {
            accessers: BTreeMap::new(),
        }
    }
    pub fn from_rule(name: &'g str, id: TokenStream) -> Self {
        let mut res = BTreeMap::new();
        res.insert(name, Node::from_rule(id));
        Self { accessers: res }
    }
    #[cfg(feature = "grammar-extras")]
    pub fn from_tag(name: &'g str, id: TokenStream) -> Self {
        let mut res = BTreeMap::new();
        res.insert(name, Node::from_tag(id));
        Self { accessers: res }
    }
    pub fn content(self) -> Self {
        self.prepend(Edge::Content)
    }
    pub fn content_i(self, i: usize) -> Self {
        self.prepend(Edge::ContentI(i))
    }
    pub fn contents(self) -> Self {
        self.prepend(Edge::Contents)
    }
    pub fn optional_content(self) -> Self {
        self.prepend(Edge::OptionalContent)
    }
    pub fn optional_first(self) -> Self {
        self.prepend(Edge::GetFirst)
    }
    pub fn optional_second(self) -> Self {
        self.prepend(Edge::GetSecond)
    }
    #[inline]
    fn prepend(mut self, edge: Edge) -> Self {
        for (_, node) in self.accessers.iter_mut() {
            // TODO: Ellide clone here.
            *node = node.clone().wrap(edge.clone());
        }
        self
    }
    pub fn join(mut self, other: Accesser<'g>) -> Self {
        other.accessers.into_iter().for_each(|(name, tree)| {
            let entry = self.accessers.entry(name);
            match entry {
                btree_map::Entry::Vacant(entry) => {
                    entry.insert(tree);
                }
                btree_map::Entry::Occupied(mut entry) => {
                    // TODO: Ellide clone here.
                    let t = entry.get_mut();
                    *t = t.clone().merge(tree);
                }
            }
        });
        self
    }
    pub fn collect(&self, root: &TokenStream) -> TokenStream {
        let accessers = self.accessers.iter().map(|(name, node)| {
            let id = ident(name);
            let (paths, types) = node.expand(root);
            let src = quote! {
                #[allow(non_snake_case)]
                pub fn #id<'s>(&'s self) -> #types {
                    let res = &self.content;
                    #paths
                }
            };
            // We may generate source codes to help debugging here.
            let doc = format! {"A helper function to access [`{}`].", name};
            quote! {
                #[doc = #doc]
                #src
            }
        });
        quote! {
            #(#accessers)*
        }
    }
}

fn position() -> TokenStream {
    let pest = pest();
    quote! {#pest::Position}
}
fn _bool() -> TokenStream {
    quote! {::core::primitive::bool}
}
fn _char() -> TokenStream {
    quote! {::core::primitive::char}
}
fn _i32() -> TokenStream {
    quote! {::core::primitive::i32}
}
fn _str() -> TokenStream {
    quote! {::core::primitive::str}
}
fn stack() -> TokenStream {
    let pest = pest();
    quote! {#pest::Stack}
}
fn tracker() -> TokenStream {
    let pest = pest();
    quote! {#pest::tracker::Tracker}
}
fn _span() -> TokenStream {
    let pest = pest();
    quote! {#pest::Span}
}

#[derive(Clone, Copy)]
enum Emission {
    /// Current rule will not contain a span.
    /// Current rule will not be visible in some APIs.
    Silent,
    /// Current rule will only contain a span.
    /// Inner structures will not be emitted.
    Span,
    /// Normal rule.
    InnerToken,
}
impl Emission {
    pub fn emit_content(&self) -> bool {
        match self {
            Emission::InnerToken | Emission::Silent => true,
            Emission::Span => false,
        }
    }
    pub fn emit_span(&self) -> bool {
        match self {
            Emission::InnerToken | Emission::Span => true,
            Emission::Silent => false,
        }
    }
}

struct RuleConfig<'g> {
    pub atomicity: Option<bool>,
    pub rule_id: Ident,
    pub rule_name: &'g str,
    pub rule_doc: Option<&'g str>,
}
impl<'g> RuleConfig<'g> {}

fn create<'g>(
    doc: impl Iterator<Item = &'g str>,
    id: &Ident,
    type_name: &TokenStream,
    rule_config: &RuleConfig,
    emission: Emission,
    accessers: TokenStream,
    root: &TokenStream,
) -> TokenStream {
    let pest_typed = pest_typed();
    let result = result_type();
    let position = position();
    let stack = stack();
    let span = _span();
    let pest = pest();
    let error = quote! {#pest::error::Error};
    let _bool = _bool();
    let str = _str();
    let tracker = tracker();
    let ignore = ignore(root);
    let rule = quote! {#root::Rule};
    let pairs = pairs();
    let box_ = box_type();

    let emit_content = emission.emit_content();
    let emit_span = emission.emit_span();
    let rule_id = &rule_config.rule_id;
    let atomicity = match rule_config.atomicity {
        Some(true) => quote! {true},
        Some(false) => quote! {false},
        None => quote! {ATOMIC},
    };

    let fields = {
        let content = if emit_content {
            quote! {
                #[doc = "Matched content."]
                pub content: #type_name,
            }
        } else {
            quote! {}
        };
        let span = if emit_span {
            quote! {
                #[doc = "Matched span."]
                pub span: #span<'i>,
            }
        } else {
            quote! {}
        };
        quote! {#content #span}
    };
    let deref = if emit_content {
        quote! {
            impl<'i> ::core::ops::Deref for #id<'i> {
                type Target = #type_name;
                fn deref(&self) -> &Self::Target {
                    &self.content
                }
            }
            impl<'i> ::core::ops::DerefMut for #id<'i> {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.content
                }
            }
        }
    } else {
        quote! {}
    };
    let pairs_impl = if emit_span {
        quote! {
            impl<'i: 'n, 'n> #pest_typed::iterators::Pairs<'i, 'n, #root::Rule> for #id<'i> {
                type Iter = ::core::iter::Once<&'n dyn #pest_typed::RuleStruct<'i, #root::Rule>>;
                type IntoIter = ::core::iter::Once<#box_<dyn #pest_typed::RuleStruct<'i, #root::Rule> + 'n>>;

                fn iter(&'n self) -> Self::Iter {
                    ::core::iter::once(self)
                }
                fn into_iter(self) -> Self::IntoIter {
                    ::core::iter::once(#box_::new(self))
                }
            }
        }
    } else {
        quote! {
            impl<'i: 'n, 'n> #pest_typed::iterators::Pairs<'i, 'n, #root::Rule> for #id<'i> {
                type Iter = <#type_name as #pest_typed::iterators::Pairs<'i, 'n, #root::Rule>>::Iter;
                type IntoIter = <#type_name as #pest_typed::iterators::Pairs<'i, 'n, #root::Rule>>::IntoIter;

                fn iter(&'n self) -> Self::Iter {
                    self.content.iter()
                }
                fn into_iter(self) -> Self::IntoIter {
                    self.content.into_iter()
                }
            }
        }
    };
    let pair_impl = if emit_content {
        quote! {
            impl<'i: 'n, 'n> #pest_typed::iterators::Pair<'i, 'n, #root::Rule> for #id<'i> {
                type Inner = <#type_name as #pest_typed::iterators::Pairs<'i, 'n, #root::Rule>>::Iter;
                type IntoInner = <#type_name as #pest_typed::iterators::Pairs<'i, 'n, #root::Rule>>::IntoIter;

                fn inner(&'n self) -> Self::Inner {
                    self.content.iter()
                }
                fn into_inner(self) -> Self::IntoInner {
                    self.content.into_iter()
                }
            }
        }
    } else {
        quote! {
            impl<'i: 'n, 'n> #pest_typed::iterators::Pair<'i, 'n, #root::Rule> for #id<'i> {
                type Inner = ::core::iter::Empty<&'n dyn #pest_typed::RuleStruct<'i, #root::Rule>>;
                type IntoInner = ::core::iter::Empty<#box_<dyn #pest_typed::RuleStruct<'i, #root::Rule> + 'n>>;

                fn inner(&'n self) -> Self::Inner {
                    ::core::iter::empty()
                }
                fn into_inner(self) -> Self::IntoInner {
                    ::core::iter::empty()
                }
            }
        }
    };
    let rule_struct_impl = {
        match emission {
            Emission::Silent => quote! {},
            Emission::Span => quote! {
                impl<'i> #pest_typed::RuleStruct<'i, #root::Rule> for #id<'i> {
                    fn span(&self) -> #span<'i> {
                        self.span
                    }
                }
            },
            Emission::InnerToken => quote! {
                impl<'i> #pest_typed::RuleStruct<'i, #root::Rule> for #id<'i> {
                    fn span(&self) -> #span<'i> {
                        self.span
                    }
                }
            },
        }
    };
    let parse_impl = match emission {
        Emission::Silent => quote! {
            let (input, content) = #type_name::try_parse_with::<#atomicity>(input, stack, tracker)?;
            Ok((input, Self { content }))
        },
        Emission::Span => quote! {
            let start = input;
            tracker.record_during(
                input,
                |tracker| {
                    let (input, _) = #type_name::try_parse_with::<#atomicity>(input, stack, tracker)?;
                    let span = start.span(&input);
                    Ok((input, Self { span }))
                }
            )
        },
        Emission::InnerToken => quote! {
            let start = input;
            tracker.record_during(
                input,
                |tracker| {
                    let (input, content) = #type_name::try_parse_with::<#atomicity>(input, stack, tracker)?;
                    let span = start.span(&input);
                    Ok((input, Self { content, span }))
                }
            )
        },
    };
    let debug_impl = {
        let rule_name = rule_config.rule_name;
        match emission {
            Emission::Silent => quote! {
                f.debug_struct(#rule_name)
                    .field("content", &self.content)
                    .finish()
            },
            Emission::Span => quote! {
                f.debug_struct(#rule_name)
                    .field("span", &self.span)
                    .finish()
            },
            Emission::InnerToken => quote! {
                f.debug_struct(#rule_name)
                    .field("content", &self.content)
                    .field("span", &self.span)
                    .finish()
            },
        }
    };
    quote! {
        #(#[doc = #doc])*
        #[allow(non_camel_case_types)]
        #[derive(Clone)]
        pub struct #id<'i> {
            #fields
        }
        impl<'i> #id<'i> {
            #accessers
        }
        #deref
        impl<'i> #pest_typed::RuleWrapper<#root::Rule> for #id<'i> {
            const RULE: #root::Rule = #root::Rule::#rule_id;
            type Rule = #root::Rule;
        }
        impl<'i> #pest_typed::TypeWrapper for #id<'i> {
            type Inner = #type_name;
        }
        impl<'i> #pest_typed::TypedNode<'i, #rule> for #id<'i> {
            #[inline]
            fn try_parse_with<const ATOMIC: #_bool>(
                input: #position<'i>,
                stack: &mut #stack<#span<'i>>,
                tracker: &mut #tracker<'i, #rule>,
            ) -> #result<(#position<'i>, Self), ()> {
                #parse_impl
            }
        }
        impl<'i> #pest_typed::ParsableTypedNode<'i, #rule> for #id<'i> {
            #[inline]
            fn parse(input: &'i #str) -> #result<Self, #error<#rule>> {
                let mut stack = #stack::new();
                let input = #position::from_start(input);
                let mut tracker = #tracker::new(input);
                let (input, res) =
                    match Self::try_parse_with::<false>(input, &mut stack, &mut tracker) {
                        Ok((input, res)) => (input, res),
                        Err(_) => return Err(tracker.collect()),
                    };
                let (input, _) = #ignore::parse_with::<false>(input, &mut stack);
                let (_, _) = match #root::#pairs::EOI::try_parse_with::<false>(input, &mut stack, &mut tracker) {
                    Ok((input, res)) => (input, res),
                    Err(_) => return Err(tracker.collect()),
                };
                Ok(res)
            }

            #[inline]
            fn parse_partial(input: &'i #str) -> #result<(#position<'i>, Self), #error<#rule>> {
                let mut stack = #stack::new();
                let input = #position::from_start(input);
                let mut tracker = #tracker::new(input);
                match Self::try_parse_with::<false>(input, &mut stack, &mut tracker) {
                    Ok((input, res)) => Ok((input, res)),
                    Err(_) => return Err(tracker.collect()),
                }
            }
        }
        impl<'i> ::core::fmt::Debug for #id<'i> {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                #debug_impl
            }
        }
        #pairs_impl
        #pair_impl
        #rule_struct_impl
    }
}

fn rule(
    rule_config: &RuleConfig,
    type_name: &TokenStream,
    doc: &String,
    accessers: &Accesser,
    emission: Emission,
) -> TokenStream {
    let root = quote! {super};
    let _bool = _bool();
    let atomicity_doc = match rule_config.atomicity {
        Some(true) => "Atomic rule.",
        Some(false) => "Non-atomic rule.",
        None => "Normal rule.",
    };
    let accessers = match emission {
        Emission::InnerToken => accessers.collect(&root),
        Emission::Silent | Emission::Span => quote! {},
    };
    let docs = [doc, atomicity_doc];
    macro_rules! create {
        ($docs:expr) => {{
            create(
                $docs,
                &rule_config.rule_id,
                type_name,
                rule_config,
                emission,
                accessers,
                &root,
            )
        }};
    }
    match rule_config.rule_doc {
        Some(rule_doc) => create!(docs.into_iter().chain(std::iter::once(rule_doc))),
        None => create!(docs.into_iter()),
    }
}

struct Output {
    content: Vec<TokenStream>,
    wrappers: Vec<TokenStream>,
    wrapper_counter: usize,
    #[cfg(feature = "grammar-extras")]
    tagged_nodes: BTreeMap<Ident, Vec<TokenStream>>,
    sequences: BTreeSet<usize>,
}
impl Output {
    fn new() -> Self {
        Self {
            content: Vec::new(),
            wrappers: Vec::new(),
            wrapper_counter: 0,
            #[cfg(feature = "grammar-extras")]
            tagged_nodes: BTreeMap::new(),
            sequences: BTreeSet::new(),
        }
    }
    /// Record usage of Seq* generics.
    fn record_seq(&mut self, index: usize) {
        self.sequences.insert(index);
    }
    /// Used sequences.
    fn seq(&self) -> &BTreeSet<usize> {
        &self.sequences
    }
    /// Insert rule struct to rule module.
    fn insert(&mut self, tokens: TokenStream) {
        self.content.push(tokens);
    }
    /// Insert tag struct to tag module.
    /// Return the module path relative to module root.
    #[cfg(feature = "grammar-extras")]
    fn insert_tag(&mut self, rule_name: &Ident, tokens: TokenStream) -> TokenStream {
        let entry = self.tagged_nodes.entry(rule_name.clone());
        match entry {
            btree_map::Entry::Vacant(entry) => {
                entry.insert(vec![tokens]);
            }
            btree_map::Entry::Occupied(mut entry) => {
                let vec = entry.get_mut();
                vec.push(tokens);
            }
        }
        quote! { tags::#rule_name }
    }
    /// Insert a string wrapper to corresponding module.
    /// Return the module path relative to module root.
    fn insert_string_wrapper(&mut self, string: &str) -> TokenStream {
        let s = ident(&format!("w_{}", self.wrapper_counter));
        self.wrapper_counter += 1;
        let doc = format!("A wrapper for `{:?}`.", string);
        let str = _str();
        let wrapper_mod = constant_wrappers();
        let pest_typed = pest_typed();
        let wrapper = quote! {
            #[doc = #doc]
            #[allow(non_camel_case_types)]
            #[derive(Clone)]
            pub struct #s();
            impl #pest_typed::StringWrapper for #s {
                const CONTENT: &'static #str = #string;
            }
        };
        self.wrappers.push(wrapper);
        quote! {#wrapper_mod::#s}
    }
    /// Insert a string array wrapper to corresponding module.
    /// Return the module path relative to module root.
    fn insert_string_array_wrapper(&mut self, strings: &[String]) -> TokenStream {
        let s = ident(&format!("w_{}", self.wrapper_counter));
        self.wrapper_counter += 1;
        let doc = format!("A wrapper for `{:?}`.", strings);
        let str = _str();
        let wrapper_mod = constant_wrappers();
        let pest_typed = pest_typed();
        let wrapper = quote! {
            #[doc = #doc]
            #[allow(non_camel_case_types)]
            #[derive(Clone)]
            pub struct #s();
            impl #pest_typed::StringArrayWrapper for #s {
                const CONTENT: &'static [&'static #str] = &[ #(#strings),* ];
            }
        };
        self.wrappers.push(wrapper);
        quote! {#wrapper_mod::#s}
    }
    /// (nodes, wrappers)
    fn collect(&self) -> TokenStream {
        let pest_typed = pest_typed();
        let content = &self.content;
        let wrappers = &self.wrappers;
        let wrapper_mod = constant_wrappers();
        #[cfg(feature = "grammar-extras")]
        let tags = self.tagged_nodes.iter().map(|(name, def)| {
            quote! {
                pub mod #name {
                    use #pest_typed::{iterators::{Pair as _, Pairs as _}, NeverFailedTypedNode as _, TypedNode as _, };
                    use core::ops::Deref as _;
                    #(#def)*
                }
            }
        });
        #[cfg(feature = "grammar-extras")]
        let mod_tags = quote! {
            mod tags {
                #(#tags)*
            }
        };
        #[cfg(not(feature = "grammar-extras"))]
        let mod_tags = quote! {};
        quote! {
            #[doc(hidden)]
            mod #wrapper_mod {
                #(#wrappers)*
            }
            #mod_tags
            #[doc = "Definitions of statically typed nodes generated by pest-generator."]
            pub mod pairs {
                use #pest_typed::iterators::{Pair as _, Pairs as _};
                #(#content)*
            }
        }
    }
}

/// Returns (type name, accesser).
fn process_single_alias<'g>(
    map: &mut Output,
    expr: &OptimizedExpr,
    rule_config: &RuleConfig,
    type_name: TokenStream,
    accessers: Accesser<'g>,
    root: &TokenStream,
    emission: Emission,
    explicit: bool,
) -> (TokenStream, Accesser<'g>) {
    if explicit {
        let rule_id = &rule_config.rule_id;
        let doc = format!("Corresponds to expression: `{}`.", expr);
        let def = rule(rule_config, &type_name, &doc, &accessers, emission);
        map.insert(def);
        let pairs = pairs();
        (quote! {#root::#pairs::#rule_id::<'i>}, accessers)
    } else {
        (type_name, accessers)
    }
}

/// Returns type name.
fn generate_graph_node<'g>(
    expr: &'g OptimizedExpr,
    rule_config: &RuleConfig,
    // From node name to type definition and implementation
    map: &mut Output,
    explicit: bool,
    emission: Emission,
    config: Config,
    root: &TokenStream,
) -> (TokenStream, Accesser<'g>) {
    let generics = generics();
    macro_rules! walk {
        ($root:expr, $var:ident) => {{
            let mut current = $root;
            let mut nodes = Vec::<&OptimizedExpr>::new();
            while let OptimizedExpr::$var(lhs, rhs) = current {
                current = rhs;
                nodes.push(&lhs);
            }
            nodes.push(current);
            nodes
        }};
    }
    // Still some compile-time information not taken.
    match expr {
        OptimizedExpr::Str(content) => {
            let wrapper = map.insert_string_wrapper(content.as_str());
            process_single_alias(
                map,
                expr,
                rule_config,
                quote! {
                    #root::#generics::Str::<'i, #root::#wrapper>
                },
                Accesser::new(),
                root,
                emission,
                explicit,
            )
        }
        OptimizedExpr::Insens(content) => {
            let wrapper = map.insert_string_wrapper(content.as_str());
            process_single_alias(
                map,
                expr,
                rule_config,
                quote! {
                    #root::#generics::Insens::<'i, #root::#wrapper>
                },
                Accesser::new(),
                root,
                emission,
                explicit,
            )
        }
        OptimizedExpr::PeekSlice(start, end) => process_single_alias(
            map,
            expr,
            rule_config,
            match end {
                Some(end) => quote! {
                    #root::#generics::PeekSlice2::<'i, #start, #end>
                },
                None => quote! {
                    #root::#generics::PeekSlice1::<'i, #start>
                },
            },
            Accesser::new(),
            root,
            emission,
            explicit,
        ),
        OptimizedExpr::Push(expr) => {
            let (inner, accesser) =
                generate_graph_node(expr, rule_config, map, false, emission, config, root);
            process_single_alias(
                map,
                expr,
                rule_config,
                quote! {
                    #root::#generics::Push::<'i, #inner>
                },
                accesser.content(),
                root,
                emission,
                explicit,
            )
        }
        OptimizedExpr::Skip(strings) => {
            let wrapper = map.insert_string_array_wrapper(strings);
            process_single_alias(
                map,
                expr,
                rule_config,
                quote! {
                    #root::#generics::Skip::<'i, #root::#wrapper>
                },
                Accesser::new(),
                root,
                emission,
                explicit,
            )
        }
        OptimizedExpr::Range(start, end) => {
            let start = start.chars().next().unwrap();
            let end = end.chars().next().unwrap();
            process_single_alias(
                map,
                expr,
                rule_config,
                quote! {
                    #root::#generics::CharRange::<'i, #start, #end>
                },
                Accesser::new(),
                root,
                emission,
                explicit,
            )
        }
        OptimizedExpr::Ident(id) => {
            let inner = ident(id);
            let pairs = pairs();
            let accessers = if config.emit_rule_reference {
                Accesser::from_rule(id, quote! {#pairs::#inner})
            } else {
                Accesser::new()
            };
            process_single_alias(
                map,
                expr,
                rule_config,
                quote! {#root::#generics::Box::<'i, #root::#pairs::#inner::<'i>>},
                accessers,
                root,
                emission,
                explicit,
            )
        }
        OptimizedExpr::PosPred(expr) => {
            let (inner, accessers) =
                generate_graph_node(expr, rule_config, map, false, emission, config, root);
            process_single_alias(
                map,
                expr,
                rule_config,
                quote! {
                    #root::#generics::Positive::<'i, #inner>
                },
                accessers.content(),
                root,
                emission,
                explicit,
            )
        }
        OptimizedExpr::NegPred(expr) => {
            // Impossible to access inner tokens.
            let (inner, _) =
                generate_graph_node(expr, rule_config, map, false, emission, config, root);
            process_single_alias(
                map,
                expr,
                rule_config,
                quote! {
                    #root::#generics::Negative::<'i, #inner>
                },
                Accesser::new(),
                root,
                emission,
                explicit,
            )
        }
        OptimizedExpr::RestoreOnErr(expr) => {
            let (inner, accessers) =
                generate_graph_node(expr, rule_config, map, false, emission, config, root);
            let accessers = accessers.content();
            process_single_alias(
                map,
                expr,
                rule_config,
                quote! {
                    #root::#generics::Restorable::<'i, #inner>
                },
                accessers,
                root,
                emission,
                explicit,
            )
        }
        OptimizedExpr::Seq(_, _) => {
            let vec = walk!(expr, Seq);
            let mut types = Vec::<TokenStream>::with_capacity(vec.len());
            let mut accesser = Accesser::new();
            for (i, expr) in vec.into_iter().enumerate() {
                let (child, acc) =
                    generate_graph_node(expr, rule_config, map, false, emission, config, root);
                types.push(child);
                accesser = accesser.join(acc.content_i(i));
            }
            let seq = format_ident!("Seq_{}", types.len());
            map.record_seq(types.len());
            process_single_alias(
                map,
                expr,
                rule_config,
                quote! {
                    #root::#generics::#seq::<
                        'i,
                        #(#types, )*
                    >
                },
                accesser,
                root,
                emission,
                explicit,
            )
        }
        OptimizedExpr::Choice(lhs, rhs) => {
            let (first, acc_first) =
                generate_graph_node(lhs, rule_config, map, false, emission, config, root);
            let acc_first = acc_first.optional_first();
            let (second, acc_second) =
                generate_graph_node(rhs, rule_config, map, false, emission, config, root);
            let acc_second = acc_second.optional_second();
            process_single_alias(
                map,
                expr,
                rule_config,
                quote! {
                    #root::#generics::Choice::<
                        'i,
                        #first,
                        #second,
                    >
                },
                acc_first.join(acc_second),
                root,
                emission,
                explicit,
            )
        }
        OptimizedExpr::Opt(inner) => {
            let (inner_name, accessers) =
                generate_graph_node(inner, rule_config, map, false, emission, config, root);
            let accessers = accessers.optional_content();
            process_single_alias(
                map,
                expr,
                rule_config,
                quote! {#root::#generics::Opt::<'i, #inner_name>},
                accessers,
                root,
                emission,
                explicit,
            )
        }
        OptimizedExpr::Rep(inner) => {
            let (inner_name, accessers) =
                generate_graph_node(inner, rule_config, map, false, emission, config, root);
            process_single_alias(
                map,
                expr,
                rule_config,
                quote! {
                    #root::#generics::Rep::<
                        'i,
                        #inner_name,
                    >
                },
                accessers.contents(),
                root,
                emission,
                explicit,
            )
        }
        #[cfg(feature = "grammar-extras")]
        OptimizedExpr::RepOnce(inner) => {
            let (inner_name, accessers) =
                generate_graph_node(inner, rule_config, map, false, emission, config, root);
            process_single_alias(
                map,
                expr,
                rule_config,
                quote! {
                    #root::#generics::RepOnce::<
                        'i,
                        #inner_name,
                    >
                },
                accessers.contents(),
                root,
                emission,
                explicit,
            )
        }
        #[cfg(feature = "grammar-extras")]
        OptimizedExpr::NodeTag(inner_expr, tag) => {
            if config.emit_tagged_node_reference {
                let new_root = &quote! {super::super};
                let tag_id = ident(tag.as_str());
                let (inner, accesser) = generate_graph_node(
                    inner_expr,
                    rule_config,
                    map,
                    explicit,
                    emission,
                    config,
                    new_root,
                );
                let def = create(
                    [format!("Tag {} referenced by {}.", tag, rule_config.rule_name).as_str()]
                        .into_iter(),
                    &tag_id,
                    &inner,
                    rule_config,
                    Emission::InnerToken,
                    accesser.collect(&new_root),
                    new_root,
                );
                let rule_id = &rule_config.rule_id;
                let tag_module = map.insert_tag(rule_id, def);
                let new_accesser =
                    Accesser::from_tag(tag.as_str(), quote! {tags::#rule_id::#tag_id});
                if config.truncate_accesser_at_node_tag {
                    (quote! {#root::#tag_module::#tag_id::<'i>}, new_accesser)
                } else {
                    (
                        quote! {#root::#tag_module::#tag_id::<'i>},
                        new_accesser.join(accesser),
                    )
                }
            } else {
                let (inner, accesser) = generate_graph_node(
                    inner_expr,
                    rule_config,
                    map,
                    explicit,
                    emission,
                    config,
                    root,
                );
                process_single_alias(
                    map,
                    expr,
                    rule_config,
                    inner,
                    accesser,
                    root,
                    emission,
                    false,
                )
            }
        }
    }
}

fn generate_graph(rules: &[OptimizedRule], config: Config, doc: &DocComment) -> Output {
    let mut res = Output::new();
    for rule in rules.iter() {
        let rule_name = rule.name.as_str();
        let (atomicity, emission) = match rule.ty {
            RuleType::Normal => (None, Emission::InnerToken),
            RuleType::Silent => (None, Emission::Silent),
            RuleType::NonAtomic => (Some(false), Emission::InnerToken),
            RuleType::CompoundAtomic => (Some(true), Emission::InnerToken),
            RuleType::Atomic => (Some(true), Emission::Span),
        };
        let rule_doc = doc.line_docs.get(rule_name).map(|s| s.as_str());
        let rule_config = RuleConfig {
            atomicity,
            rule_id: ident(rule_name),
            rule_name,
            rule_doc,
        };
        generate_graph_node(
            &rule.expr,
            &rule_config,
            &mut res,
            true,
            emission,
            config,
            &quote! {super},
        );
    }
    res
}

fn collect_used_rule<'s>(rule: &'s OptimizedRule, res: &mut BTreeSet<&'s str>) {
    let mut exprs = vec![&rule.expr];
    while let Some(expr) = exprs.pop() {
        match expr {
            OptimizedExpr::Str(_) | OptimizedExpr::Insens(_) | OptimizedExpr::Range(_, _) => (),
            OptimizedExpr::Ident(rule_name) => {
                res.insert(rule_name.as_str());
            }
            OptimizedExpr::PeekSlice(_, _) => (),
            OptimizedExpr::PosPred(expr) | OptimizedExpr::NegPred(expr) => exprs.push(expr),
            OptimizedExpr::Seq(lhs, rhs) | OptimizedExpr::Choice(lhs, rhs) => {
                exprs.push(lhs);
                exprs.push(rhs);
            }
            OptimizedExpr::Opt(expr) | OptimizedExpr::Rep(expr) => exprs.push(expr),
            #[cfg(feature = "grammar-extras")]
            OptimizedExpr::RepOnce(expr) => exprs.push(expr),
            OptimizedExpr::Skip(_) => (),
            OptimizedExpr::Push(expr) | OptimizedExpr::RestoreOnErr(expr) => exprs.push(expr),
            #[cfg(feature = "grammar-extras")]
            OptimizedExpr::NodeTag(expr, _) => exprs.push(expr),
        }
    }
}

pub(crate) fn generate_typed_pair_from_rule(
    rules: &[OptimizedRule],
    doc: &DocComment,
    config: Config,
) -> TokenStream {
    let pest_typed = pest_typed();
    let mut graph = generate_graph(rules, config, doc);
    let as_wrapper = |name: &Ident| {
        quote! {
            #[allow(non_camel_case_types)]
            #[derive(Clone)]
            pub struct #name;
            impl #pest_typed::RuleWrapper<super::Rule> for #name {
                const RULE: super::Rule = super::Rule::#name;
                type Rule = super::Rule;
            }
        }
    };
    let rule_wrappers = rules.iter().map(|rule| {
        let name = ident(rule.name.as_str());
        as_wrapper(&name)
    });
    let eoi = as_wrapper(&ident("EOI"));
    let defined_rules: BTreeSet<&str> = rules.iter().map(|rule| rule.name.as_str()).collect();
    let builtin = generate_builtin(&defined_rules);
    graph.insert(quote! {
        use #pest_typed::NeverFailedTypedNode as _;
        #builtin
    });
    let mods = graph.collect();
    let unicode = unicode_mod();
    let referenced_rules = {
        let mut res = BTreeSet::new();
        for rule in rules {
            collect_used_rule(rule, &mut res)
        }
        res
    };
    let unicode_rule = generate_unicode(&defined_rules, &referenced_rules);
    let generics = {
        let root = quote! {super};
        let pairs = pairs();
        let _i32 = _i32();
        let char = _char();
        let mut seq = vec![];
        for sequence in graph.seq() {
            let type_i = format_ident!("Seq_{}", sequence);
            let generics_i = format_ident!("Seq{}", sequence);
            let (types, field): (Vec<_>, Vec<_>) = (0..*sequence)
                .map(|i| (format_ident!("T{}", i), Index::from(i)))
                .unzip();
            if *sequence >= 8 {
                seq.push(quote! {
                    pest_typed::seq!(#generics_i, pest_typed, #(#types, #field, )*);
                });
            } else {
                seq.push(quote! {
                    use pest_typed::sequence::#generics_i;
                })
            }
            seq.push(quote!{
                pub type #type_i<'i, #(#types: TypedNode<'i, #root::Rule>, )*> = #generics_i<'i, #root::Rule, #(#types, )* Ignored<'i>>;
            });
        }
        quote! {
            #[doc(hidden)]
            mod generics {
                use #pest_typed as pest_typed;
                use #pest_typed::{NeverFailedTypedNode, predefined_node, StringArrayWrapper, StringWrapper, TypedNode};
                pub type Ignored<'i> = predefined_node::Ign::<
                    'i,
                    #root::Rule,
                    #root::#pairs::COMMENT::<'i>,
                    #root::#pairs::WHITESPACE::<'i>,
                >;
                pub type Str<'i, Wrapper: StringWrapper> = predefined_node::Str::<'i, #root::Rule, Wrapper>;
                pub type Insens<'i, Wrapper: StringWrapper> = predefined_node::Insens::<'i, #root::Rule, Wrapper>;
                pub type PeekSlice2<'i, const START: #_i32, const END: #_i32> = predefined_node::PeekSlice2::<'i, #root::Rule, START, END>;
                pub type PeekSlice1<'i, const START: #_i32> = predefined_node::PeekSlice1::<'i, #root::Rule, START>;
                pub type Push<'i, T: TypedNode<'i, #root::Rule>> = predefined_node::Push<'i, #root::Rule, T>;
                pub type Skip<'i, Strings: StringArrayWrapper> = predefined_node::Skip::<'i, #root::Rule, Strings>;
                pub type CharRange<'i, const START: #char, const END: #char> = predefined_node::CharRange::<'i, #root::Rule, START, END>;
                pub type Box<'i, T: TypedNode<'i, #root::Rule>> = predefined_node::Box<'i, #root::Rule, T>;
                pub type Positive<'i, T: TypedNode<'i, #root::Rule>> = predefined_node::Positive<'i, #root::Rule, T>;
                pub type Negative<'i, T: TypedNode<'i, #root::Rule>> = predefined_node::Negative<'i, #root::Rule, T>;
                pub type Restorable<'i, T: TypedNode<'i, #root::Rule>> = predefined_node::Restorable<'i, #root::Rule, T>;
                #(#seq)*
                pub type Choice<'i, T1: TypedNode<'i, #root::Rule>, T2: TypedNode<'i, #root::Rule>> = predefined_node::Choice<'i, #root::Rule, T1, T2>;
                pub type Opt<'i, T: TypedNode<'i, #root::Rule>> = predefined_node::Opt<'i, #root::Rule, T>;
                pub type Rep<'i, T: TypedNode<'i, #root::Rule>> = predefined_node::Rep<'i, #root::Rule, T, Ignored<'i>>;
                pub type RepOnce<'i, T: TypedNode<'i, #root::Rule>> = predefined_node::RepOnce<'i, #root::Rule, T, Ignored<'i>>;
            }
        }
    };
    let res = quote! {
        #[doc(hidden)]
        mod rule_wrappers {
            #(#rule_wrappers)*
            #eoi
        }
        #[doc(hidden)]
        mod #unicode {
            #unicode_rule
        }
        #mods
        #generics
    };
    res
}

fn generate_unicode(rule_names: &BTreeSet<&str>, referenced: &BTreeSet<&str>) -> TokenStream {
    let mut results = vec![];
    let pest_typed = pest_typed();
    let pest_unicode = pest_unicode();
    let bool = _bool();
    let result = result_type();
    let position = position();
    let stack = stack();
    let span = _span();
    let char = _char();
    let tracker = tracker();
    let root = quote! {super};
    let box_ = box_type();

    for property in unicode_property_names() {
        let property_ident: Ident = syn::parse_str(property).unwrap();
        // insert manually for #property substitution

        let doc = format!("Auto generated. Unicode property {}.", property);

        if !rule_names.contains(property) && referenced.contains(property) {
            results.push(quote! {
                #[allow(non_camel_case_types)]
                #[doc = #doc]
                #[derive(Clone)]
                pub struct #property_ident<'i> {
                    pub content: #char,
                    _phantom: ::core::marker::PhantomData<&'i #char>
                }
                impl<'i> ::core::convert::From<#char> for #property_ident<'i> {
                    fn from(content: #char) -> Self {
                        Self {
                            content,
                            _phantom: ::core::marker::PhantomData
                        }
                    }
                }
                impl<'i> #pest_typed::TypedNode<'i, super::Rule> for #property_ident<'i> {
                    #[inline]
                    fn try_parse_with<const ATOMIC: #bool>(
                        mut input: #position<'i>,
                        _stack: &mut #stack<#span<'i>>,
                        tracker: &mut #tracker<'i, super::Rule>,
                    ) -> #result<(#position<'i>, Self), ()> {
                        match #pest_typed::predefined_node::match_char_by(&mut input, #pest_unicode::#property_ident) {
                            Some(content) => {
                                Ok((input, Self::from(content)))
                            }
                            None => {
                                Err(())
                            }
                        }
                    }
                }
                impl<'i> ::core::fmt::Debug for #property_ident<'i> {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        f.debug_struct(#property)
                            .field("content", &self.content)
                            .finish()
                    }
                }
                impl<'i: 'n, 'n> #pest_typed::iterators::Pairs<'i, 'n, #root::Rule> for #property_ident<'i> {
                    type Iter = ::core::iter::Empty<&'n dyn #pest_typed::RuleStruct<'i, #root::Rule>>;
                    type IntoIter = ::core::iter::Empty<#box_<dyn #pest_typed::RuleStruct<'i, #root::Rule> + 'n>>;

                    fn iter(&'n self) -> Self::Iter {
                        ::core::iter::empty()
                    }
                    fn into_iter(self) -> Self::IntoIter {
                        ::core::iter::empty()
                    }
                }
            });
        }
    }
    quote! {
        #(#results)*
    }
}

fn generate_builtin(rule_names: &BTreeSet<&str>) -> TokenStream {
    let pest_typed = pest_typed();
    let unicode = unicode_mod();
    let rule_wrappers = rule_wrappers();
    let mut results = vec![quote! {
        use #pest_typed::TypedNode as _;
        use ::core::ops::Deref as _;
        use super::#unicode::*;
    }];
    macro_rules! insert_builtin {
        ($name:expr, $def:path) => {
            if !rule_names.contains($name) {
                let id = ident($name);
                results.push(quote! {
                    #[allow(non_camel_case_types)]
                    pub type #id<'i> = #pest_typed::predefined_node::$def;
                });
            }
        };
    }

    results.push(quote! {
        #[allow(non_camel_case_types)]
        pub type EOI<'i> = #pest_typed::predefined_node::AtomicRule::<'i, super::Rule, #pest_typed::predefined_node::EOI::<'i>, super::#rule_wrappers::EOI, super::#rule_wrappers::EOI>;
    });

    insert_builtin!("ANY", ANY::<'i>);
    insert_builtin!("SOI", SOI::<'i>);
    insert_builtin!("PEEK", PEEK::<'i>);
    insert_builtin!("PEEK_ALL", PEEK_ALL::<'i>);
    insert_builtin!("POP", POP::<'i>);
    insert_builtin!("POP_ALL", POP_ALL::<'i>);
    insert_builtin!("DROP", DROP::<'i>);
    insert_builtin!("ASCII_DIGIT", ASCII_DIGIT::<'i, super::Rule>);
    insert_builtin!(
        "ASCII_NONZERO_DIGIT",
        ASCII_NONZERO_DIGIT::<'i, super::Rule>
    );
    insert_builtin!("ASCII_BIN_DIGIT", ASCII_BIN_DIGIT::<'i, super::Rule>);
    insert_builtin!("ASCII_OCT_DIGIT", ASCII_OCT_DIGIT::<'i, super::Rule>);
    insert_builtin!("ASCII_HEX_DIGIT", ASCII_HEX_DIGIT::<'i, super::Rule>);
    insert_builtin!("ASCII_ALPHA_LOWER", ASCII_ALPHA_LOWER::<'i, super::Rule>);
    insert_builtin!("ASCII_ALPHA_UPPER", ASCII_ALPHA_UPPER::<'i, super::Rule>);
    insert_builtin!("ASCII_ALPHA", ASCII_ALPHA::<'i, super::Rule>);
    insert_builtin!("ASCII_ALPHANUMERIC", ASCII_ALPHANUMERIC::<'i, super::Rule>);
    insert_builtin!("ASCII", ASCII::<'i, super::Rule>);
    insert_builtin!("NEWLINE", NEWLINE::<'i>);

    insert_builtin!("WHITESPACE", AlwaysFail::<'i>);
    insert_builtin!("COMMENT", AlwaysFail::<'i>);

    quote! {
        #(#results)*
    }
}
