// pest-typed. A statically typed version of pest.
// Copyright (c) 2023 黄博奕
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

use core::fmt::Debug;

pub use alloc::rc::Rc;
use pest::RuleType;

use super::{
    error::Error, position::Position, span::Span, stack::Stack, tracker::Tracker,
    wrapper::RuleWrapper,
};

/// Node of concrete syntax tree that never fails.
pub trait NeverFailedTypedNode<'i, R: RuleType>
where
    Self: Sized + Debug + Clone,
{
    /// Create typed node.
    /// `ATOMIC` refers to the external status, and it can be overriden by rule definition.
    fn parse_with<const ATOMIC: bool, Rule: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> (Position<'i>, Self);
}

/// Node of concrete syntax tree.
pub trait TypedNode<'i, R: RuleType>
where
    Self: Sized + Debug + Clone,
{
    /// Create typed node.
    /// `ATOMIC` refers to the external status, and it can be overriden by rule definition.
    fn try_parse_with<const ATOMIC: bool, Rule: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>>;
}

/// Node of concrete syntax tree.
pub trait ParsableTypedNode<'i, R: RuleType>: TypedNode<'i, R> {
    /// Parse the whole input into given typed node.
    /// A rule is not atomic by default.
    fn parse(input: &'i str) -> Result<Self, Error<R>>;
    /// Parse the whole input into given typed node.
    /// A rule is not atomic by default.
    fn parse_partial(input: &'i str) -> Result<(Position<'i>, Self), Error<R>>;
}
