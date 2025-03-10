// pest-typed. A statically typed version of pest.
// Copyright (c) 2023 黄博奕
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

use crate::{
    error::Error, position::Position, predefined_node::restore_on_none, span::Span,
    tracker::Tracker, RuleWrapper, Stack,
};
use alloc::{boxed::Box, vec::Vec};
use core::fmt::Debug;
use pest::RuleType;

/// Node of concrete syntax tree that never fails.
pub trait NeverFailedTypedNode<'i, R: RuleType>
where
    Self: Sized + Debug + Clone + PartialEq + Default,
{
    /// Create typed node.
    /// `ATOMIC` refers to the external status, and it can be overriden by rule definition.
    fn parse_with(input: Position<'i>, stack: &mut Stack<Span<'i>>) -> (Position<'i>, Self);
}

/// Node of concrete syntax tree.
pub trait TypedNode<'i, R: RuleType>
where
    Self: Sized + Debug + Clone + PartialEq,
{
    /// Create typed node.
    fn try_parse_with(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
        tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)>;
}

/// Node of concrete syntax tree.
pub trait ParsableTypedNode<'i, R: RuleType>: TypedNode<'i, R> {
    /// Try to create typed node.
    fn try_parse_with_until_end(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
        tracker: &mut Tracker<'i, R>,
    ) -> Option<Self>;
    /// Try to parse the whole input into given typed node.
    /// A rule is not atomic by default.
    fn try_parse(input: &'i str) -> Result<Self, Box<Error<R>>> {
        let mut stack = Stack::new();
        let input = Position::from_start(input);
        let mut tracker = Tracker::new(input);
        match Self::try_parse_with_until_end(input, &mut stack, &mut tracker) {
            Some(res) => Ok(res),
            None => Err(Box::new(tracker.collect())),
        }
    }
    /// Try to parse the whole input into given typed node.
    /// A rule is not atomic by default.
    fn try_parse_partial(input: &'i str) -> Result<(Position<'i>, Self), Box<Error<R>>> {
        let mut stack = Stack::new();
        let input = Position::from_start(input);
        let mut tracker = Tracker::new(input);
        match Self::try_parse_with(input, &mut stack, &mut tracker) {
            Some((input, res)) => Ok((input, res)),
            None => Err(Box::new(tracker.collect())),
        }
    }
}

/// Node of concrete syntax tree.
pub trait NeverFailedParsableTypedNode<'i, R: RuleType>: NeverFailedTypedNode<'i, R> {
    /// Create typed node.
    fn parse_with_until_end(input: Position<'i>, stack: &mut Stack<Span<'i>>) -> Self;
    /// Parse the whole input into given typed node.
    /// A rule is not atomic by default.
    fn parse(input: &'i str) -> Self {
        let mut stack = Stack::new();
        let input = Position::from_start(input);
        Self::parse_with_until_end(input, &mut stack)
    }
    /// Parse the whole input into given typed node.
    /// A rule is not atomic by default.
    fn parse_partial(input: &'i str) -> (Position<'i>, Self) {
        let mut stack = Stack::new();
        let input = Position::from_start(input);
        Self::parse_with(input, &mut stack)
    }
}

pub trait RuleStorage<R: RuleType> {
    fn rule(&self) -> R;
}
impl<R: RuleType, T: RuleWrapper<R>> RuleStorage<R> for T {
    fn rule(&self) -> R {
        T::RULE
    }
}

/// A trait for those struct that correspond to non-silent rules.
pub trait Spanned<'i, R: RuleType> {
    /// The span of a matched expression by a non-silent rule.
    fn span(&self) -> Span<'i>;
}

/// A trait for those struct that correspond to rules with inner expression.
pub trait RuleStruct<'i, R: RuleType>: RuleStorage<R> {
    /// Inner type.
    type Inner;
    /// Take inner content.
    fn take_inner(self) -> Self::Inner;
    /// Reference inner content.
    fn ref_inner(&self) -> &Self::Inner;
    /// Reference inner content mutably.
    fn mut_inner(&mut self) -> &mut Self::Inner;
}

/// Match `[T; N]`.
impl<'i, R: RuleType, T: TypedNode<'i, R>, const N: usize> TypedNode<'i, R> for [T; N] {
    fn try_parse_with(
        mut input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
        tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        let mut vec = Vec::new();
        for _ in 0..N {
            let (next, res) = T::try_parse_with(input, stack, tracker)?;
            input = next;
            vec.push(res);
        }
        match vec.try_into() {
            Ok(res) => Some((input, res)),
            // Actually impossible.
            Err(_) => None,
        }
    }
}

/// Match `(T1, T2)`.
impl<'i, R: RuleType, T1: TypedNode<'i, R>, T2: TypedNode<'i, R>> TypedNode<'i, R> for (T1, T2) {
    #[inline]
    fn try_parse_with(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
        tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        let (input, t1) = T1::try_parse_with(input, stack, tracker)?;
        let (input, t2) = T2::try_parse_with(input, stack, tracker)?;
        Some((input, (t1, t2)))
    }
}

/// Optionally match `T`.
impl<'i, R: RuleType, T: TypedNode<'i, R>> TypedNode<'i, R> for Option<T> {
    #[inline]
    fn try_parse_with(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
        tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        let res = restore_on_none(stack, |stack| T::try_parse_with(input, stack, tracker));
        match res {
            Some((input, inner)) => Some((input, Some(inner))),
            None => Some((input, None)),
        }
    }
}
