// pest-typed. A statically typed version of pest.
// Copyright (c) 2023 黄博奕
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Predefined tree nodes.

use core::{fmt, fmt::Debug, marker::PhantomData};

use alloc::vec::Vec;

use pest::RuleType;

use super::{error::Error, parser_state::constrain_idxs, position::Position, stack::Stack};

use super::{
    span::Span,
    tracker::Tracker,
    typed_node::{NeverFailedTypedNode, ParsableTypedNode},
    wrapper::{RuleWrapper, StringArrayWrapper, StringWrapper, TypeWrapper},
    TypedNode,
};

/// Match given string.
pub struct Str<'i, R: RuleType, T: StringWrapper> {
    _phantom: PhantomData<(&'i R, &'i T)>,
}
impl<'i, R: RuleType, T: StringWrapper> StringWrapper for Str<'i, R, T> {
    const CONTENT: &'static str = T::CONTENT;
}
impl<'i, R: RuleType, T: StringWrapper> From<()> for Str<'i, R, T> {
    fn from(_value: ()) -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}
impl<'i, R: RuleType, T: StringWrapper> TypedNode<'i, R> for Str<'i, R, T> {
    fn try_parse_with<const _A: bool, Rule: RuleWrapper<R>>(
        mut input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        if input.match_string(Self::CONTENT) {
            Ok((input, Self::from(())))
        } else {
            Err(Tracker::new(input))
        }
    }
}
impl<'i, R: RuleType, T: StringWrapper> Debug for Str<'i, R, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Str").finish()
    }
}

/// Match given string case insensitively.
pub struct Insens<'i, R: RuleType, T: StringWrapper> {
    /// Matched content.
    pub content: &'i str,
    _phantom: PhantomData<(&'i R, &'i T)>,
}
impl<'i, R: RuleType, T: StringWrapper> StringWrapper for Insens<'i, R, T> {
    const CONTENT: &'static str = T::CONTENT;
}
impl<'i, R: RuleType, T: StringWrapper> From<&'i str> for Insens<'i, R, T> {
    fn from(content: &'i str) -> Self {
        Self {
            content,
            _phantom: PhantomData,
        }
    }
}
impl<'i, R: RuleType, T: StringWrapper> TypedNode<'i, R> for Insens<'i, R, T> {
    fn try_parse_with<const _A: bool, Rule: RuleWrapper<R>>(
        mut input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        let start = input.clone();
        if input.match_insensitive(Self::CONTENT) {
            let span = start.span(&input);
            Ok((input, Self::from(span.as_str())))
        } else {
            Err(Tracker::new_positive(Rule::RULE, input))
        }
    }
}
impl<'i, R: RuleType, T: StringWrapper> Debug for Insens<'i, R, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Insens").finish()
    }
}

/// Inner tokens will be discarded, and only a [`Span`] will be contained.
///
/// And inner errors will **not** be tracked.
pub struct Silent<'i, R: RuleType, T: TypedNode<'i, R>> {
    /// Span.
    pub span: Span<'i>,
    _phantom: PhantomData<(&'i R, &'i T)>,
}
impl<'i, R: RuleType, T: TypedNode<'i, R>> TypedNode<'i, R> for Silent<'i, R, T> {
    fn try_parse_with<const ATOMIC: bool, Rule: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        let start = input.clone();
        match T::try_parse_with::<ATOMIC, Rule>(input, stack) {
            Ok((input, _)) => {
                let span = start.span(&input);
                Ok((
                    input,
                    Self {
                        span,
                        _phantom: PhantomData,
                    },
                ))
            }
            Err(tracker) => Err(Tracker::new_positive(Rule::RULE, tracker.position())),
        }
    }
}
impl<'i, R: RuleType, T: TypedNode<'i, R>> Debug for Silent<'i, R, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Silent").finish()
    }
}

/// Skips until one of the given `strings`
pub struct Skip<'i, R: RuleType, Strings: StringArrayWrapper> {
    /// Skipped span.
    pub span: Span<'i>,
    _phantom: PhantomData<(&'i R, &'i Strings)>,
}
impl<'i, R: RuleType, Strings: StringArrayWrapper> TypedNode<'i, R> for Skip<'i, R, Strings> {
    fn try_parse_with<const _A: bool, Rule: RuleWrapper<R>>(
        mut input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        let start = input.clone();
        match input.skip_until(Strings::CONTENT) {
            true => {
                let span = start.span(&input);
                Ok((
                    input,
                    Self {
                        span,
                        _phantom: PhantomData,
                    },
                ))
            }
            false => Err(Tracker::new_positive(Rule::RULE, input)),
        }
    }
}
impl<'i, R: RuleType, Strings: StringArrayWrapper> Debug for Skip<'i, R, Strings> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Skip").finish()
    }
}

/// Skip `n` characters if there are.
pub struct SkipChar<'i, R: RuleType, const N: usize> {
    _phantom: PhantomData<&'i R>,
}
impl<'i, R: RuleType, const N: usize> From<()> for SkipChar<'i, R, N> {
    fn from(_: ()) -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}
impl<'i, R: RuleType, const N: usize> TypedNode<'i, R> for SkipChar<'i, R, N> {
    fn try_parse_with<const ATOMIC: bool, Rule: RuleWrapper<R>>(
        mut input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        match input.skip(N) {
            true => Ok((input, Self::from(()))),
            false => Err(Tracker::new(input)),
        }
    }
}
impl<'i, R: RuleType, const N: usize> Debug for SkipChar<'i, R, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Range").finish()
    }
}

/// Match a character in the range `[min, max]`.
/// Inclusively both below and above.
pub struct CharRange<'i, R: RuleType, const MIN: char, const MAX: char> {
    /// Matched character.
    pub content: char,
    _phantom: PhantomData<&'i R>,
}

impl<'i, R: RuleType, const MIN: char, const MAX: char> TypedNode<'i, R>
    for CharRange<'i, R, MIN, MAX>
{
    fn try_parse_with<const _A: bool, Rule: RuleWrapper<R>>(
        mut input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        let start = input.clone();
        match input.match_range(MIN..MAX) {
            true => {
                let span = start.span(&input);
                let content = span.as_str().chars().next().unwrap();
                Ok((
                    input,
                    Self {
                        content,
                        _phantom: PhantomData,
                    },
                ))
            }
            false => Err(Tracker::new_positive(Rule::RULE, input)),
        }
    }
}

impl<'i, R: RuleType, const MIN: char, const MAX: char> Debug for CharRange<'i, R, MIN, MAX> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Range")
            .field("content", &self.content)
            .finish()
    }
}

/// Try to create stack slice.
#[inline]
fn stack_slice<'i, 's, R: RuleType>(
    input: Position<'i>,
    start: i32,
    end: Option<i32>,
    stack: &'s Stack<Span<'i>>,
) -> Result<core::slice::Iter<'s, Span<'i>>, Tracker<'i, R>> {
    let range = match constrain_idxs(start, end, stack.len()) {
        Some(range) => range,
        None => return Err(Tracker::SliceOutOfBound(start, end, input)),
    };
    // return true if an empty sequence is requested
    if range.end <= range.start {
        return Ok(core::slice::Iter::default());
    }
    Ok(stack[range].iter())
}

/// Match a part of the stack.
/// Will match (consume) input.
#[inline]
fn peek_spans<'s, 'i: 's, R: RuleType, Rule: RuleWrapper<R>>(
    input: Position<'i>,
    iter: impl Iterator<Item = &'s Span<'i>>,
) -> Result<(Position<'i>, Span<'i>), Tracker<'i, R>> {
    let mut matching_pos = input.clone();
    for span in iter {
        match matching_pos.match_string(span.as_str()) {
            true => (),
            false => return Err(Tracker::new(input)),
        }
    }
    Ok((matching_pos, input.span(&matching_pos)))
}

/// Positive predicate.
pub struct Positive<'i, R: RuleType, N: TypedNode<'i, R>> {
    /// Mathed content.
    pub content: N,
    _phantom: PhantomData<(&'i R, &'i N)>,
}
impl<'i, R: RuleType, N: TypedNode<'i, R>> TypedNode<'i, R> for Positive<'i, R, N> {
    fn try_parse_with<const ATOMIC: bool, Rule: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        stack.snapshot();
        match N::try_parse_with::<ATOMIC, Rule>(input, stack) {
            Ok((_input, content)) => {
                stack.restore();
                Ok((
                    input,
                    Self {
                        content,
                        _phantom: PhantomData,
                    },
                ))
            }
            Err(_) => {
                stack.restore();
                Err(Tracker::new_positive(Rule::RULE, input))
            }
        }
    }
}
impl<'i, R: RuleType, N: TypedNode<'i, R>> Debug for Positive<'i, R, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Positive").finish()
    }
}

/// Negative predicate.
pub struct Negative<'i, R: RuleType, N: TypedNode<'i, R>> {
    _phantom: PhantomData<(&'i R, &'i N)>,
}
impl<'i, R: RuleType, N: TypedNode<'i, R>> TypedNode<'i, R> for Negative<'i, R, N> {
    fn try_parse_with<const ATOMIC: bool, Rule: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        stack.snapshot();
        match N::try_parse_with::<ATOMIC, Rule>(input, stack) {
            Ok(_) => {
                stack.restore();
                Err(Tracker::new_negative(Rule::RULE, input))
            }
            Err(_) => {
                stack.restore();
                Ok((
                    input,
                    Self {
                        _phantom: PhantomData,
                    },
                ))
            }
        }
    }
}
impl<'i, R: RuleType, N: TypedNode<'i, R>> Debug for Negative<'i, R, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Negative").finish()
    }
}

/// Match any character.
#[derive(Debug)]
pub struct ANY<'i> {
    /// Pair span.
    pub span: Span<'i>,
    /// Matched character.
    pub content: char,
}
impl<'i, R: RuleType> TypedNode<'i, R> for ANY<'i> {
    #[inline]
    fn try_parse_with<const _A: bool, Rule: RuleWrapper<R>>(
        mut input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        let original_input = input.clone();
        let mut c: char = ' ';
        match input.match_char_by(|ch| {
            c = ch;
            true
        }) {
            true => {
                let span = original_input.span(&input);
                Ok((input, Self { span, content: c }))
            }
            false => Err(Tracker::new_positive(Rule::RULE, input)),
        }
    }
}

/// Match start of input.
pub struct SOI<'i> {
    _phantom: PhantomData<&'i str>,
}
impl<'i, R: RuleType> TypedNode<'i, R> for SOI<'i> {
    #[inline]
    fn try_parse_with<const _A: bool, Rule: RuleWrapper<R>>(
        input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        if input.at_start() {
            Ok((
                input,
                Self {
                    _phantom: PhantomData,
                },
            ))
        } else {
            Err(Tracker::new_positive(Rule::RULE, input))
        }
    }
}
impl<'i> Debug for SOI<'i> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SOI").finish()
    }
}

/// Match end of input.
///
/// [`EOI`] will record its rule if not matched.
pub struct EOI<'i> {
    _phantom: PhantomData<&'i str>,
}
impl<'i, R: RuleType> TypedNode<'i, R> for EOI<'i> {
    #[inline]
    fn try_parse_with<const _A: bool, Rule: RuleWrapper<R>>(
        input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        if input.at_end() {
            Ok((
                input,
                Self {
                    _phantom: PhantomData,
                },
            ))
        } else {
            Err(Tracker::new_positive(Rule::RULE, input))
        }
    }
}
impl<'i> Debug for EOI<'i> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EOI").finish()
    }
}

/// Match a new line.
#[derive(Debug)]
pub struct NEWLINE<'i> {
    /// Pair span.
    pub span: Span<'i>,
}
impl<'i, R: RuleType> TypedNode<'i, R> for NEWLINE<'i> {
    #[inline]
    fn try_parse_with<const _A: bool, Rule: RuleWrapper<R>>(
        mut input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        let start = input.clone();
        if input.match_string("\r\n") {
            let span = start.span(&input);
            Ok((input, Self { span }))
        } else if input.match_string("\n") {
            let span = start.span(&input);
            Ok((input, Self { span }))
        } else if input.match_string("\r") {
            let span = start.span(&input);
            Ok((input, Self { span }))
        } else {
            Err(Tracker::new_positive(Rule::RULE, input))
        }
    }
}

/// Peek all reversely in stack.
/// Will consume input.
#[allow(non_camel_case_types)]
#[derive(Debug)]
pub struct PEEK_ALL<'i> {
    /// Pair span.
    pub span: Span<'i>,
}
impl<'i, R: RuleType> TypedNode<'i, R> for PEEK_ALL<'i> {
    #[inline]
    fn try_parse_with<const _A: bool, Rule: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        let spans = stack[0..stack.len()].iter().rev();
        let (input, span) = peek_spans::<R, Rule>(input, spans)?;
        Ok((input, Self { span }))
    }
}

/// Peek all in stack.
/// Will consume input.
#[allow(non_camel_case_types)]
#[derive(Debug)]
pub struct PEEK<'i> {
    /// Pair span.
    pub span: Span<'i>,
}
impl<'i> From<Span<'i>> for PEEK<'i> {
    fn from(span: Span<'i>) -> Self {
        Self { span }
    }
}
impl<'i, R: RuleType> TypedNode<'i, R> for PEEK<'i> {
    #[inline]
    fn try_parse_with<const _A: bool, Rule: RuleWrapper<R>>(
        mut input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        let start = input.clone();
        match stack.peek() {
            Some(string) => match input.match_string(string.as_str()) {
                true => Ok((input, Self::from(start.span(&input)))),
                false => Err(Tracker::new(input)),
            },
            None => Err(Tracker::EmptyStack(input)),
        }
    }
}

/// Optionally match `T`.
pub struct Opt<'i, R: RuleType, T: TypedNode<'i, R>> {
    /// Matched content.
    pub content: Option<T>,
    _phantom: PhantomData<&'i R>,
}
impl<'i, R: RuleType, T: TypedNode<'i, R>> TypedNode<'i, R> for Opt<'i, R, T> {
    #[inline]
    fn try_parse_with<const ATOMIC: bool, Rule: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        match T::try_parse_with::<ATOMIC, Rule>(input, stack) {
            Ok((input, inner)) => Ok((
                input,
                Self {
                    content: Some(inner),
                    _phantom: PhantomData,
                },
            )),
            Err(_err) => Ok((
                input,
                Self {
                    content: None,
                    _phantom: PhantomData,
                },
            )),
        }
    }
}
impl<'i, R: RuleType, T: TypedNode<'i, R>> Debug for Opt<'i, R, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Opt")
            .field("content", &self.content)
            .finish()
    }
}

/// Ignore comments or white spaces if there is any.
/// Never fail.
pub struct Ign<'i, R: RuleType, COMMENT: TypedNode<'i, R>, WHITESPACE: TypedNode<'i, R>> {
    _phantom: PhantomData<(&'i R, &'i COMMENT, &'i WHITESPACE)>,
}

impl<'i, R: RuleType, COMMENT: TypedNode<'i, R>, WHITESPACE: TypedNode<'i, R>>
    NeverFailedTypedNode<'i, R> for Ign<'i, R, COMMENT, WHITESPACE>
{
    #[inline]
    fn parse_with<const ATOMIC: bool, Rule: RuleWrapper<R>>(
        mut input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> (Position<'i>, Self) {
        if ATOMIC {
            return (
                input,
                Self {
                    _phantom: PhantomData,
                },
            );
        }
        let mut flag = true;
        while flag {
            flag = false;
            while let Ok((remained, _)) = WHITESPACE::try_parse_with::<true, Rule>(input, stack) {
                input = remained;
                flag = true;
            }
            while let Ok((remained, _)) = COMMENT::try_parse_with::<true, Rule>(input, stack) {
                input = remained;
                flag = true;
            }
        }
        (
            input,
            Self {
                _phantom: PhantomData,
            },
        )
    }
}
impl<'i, R: RuleType, COMMENT: TypedNode<'i, R>, WHITESPACE: TypedNode<'i, R>> TypedNode<'i, R>
    for Ign<'i, R, COMMENT, WHITESPACE>
{
    #[inline]
    fn try_parse_with<const ATOMIC: bool, RULE: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        Ok(Self::parse_with::<ATOMIC, RULE>(input, stack))
    }
}
impl<'i, R: RuleType, COMMENT: TypedNode<'i, R>, WHITESPACE: TypedNode<'i, R>> Debug
    for Ign<'i, R, COMMENT, WHITESPACE>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ign").finish()
    }
}

/// Match a sequence of two expressions.
pub struct Seq<
    'i,
    R: RuleType,
    T1: TypedNode<'i, R>,
    T2: TypedNode<'i, R>,
    IGNORED: NeverFailedTypedNode<'i, R>,
> {
    /// Matched first expression.
    pub first: T1,
    /// Matched second expression.
    pub second: T2,
    _phantom: PhantomData<(&'i R, &'i IGNORED)>,
}
impl<
        'i,
        R: RuleType,
        T1: TypedNode<'i, R>,
        T2: TypedNode<'i, R>,
        IGNORED: NeverFailedTypedNode<'i, R>,
    > TypedNode<'i, R> for Seq<'i, R, T1, T2, IGNORED>
{
    #[inline]
    fn try_parse_with<const ATOMIC: bool, Rule: RuleWrapper<R>>(
        mut input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        let (next, first) = T1::try_parse_with::<ATOMIC, Rule>(input, stack)?;
        input = next;
        let (next, _) = IGNORED::parse_with::<ATOMIC, Rule>(input, stack);
        input = next;
        let (next, second) = T2::try_parse_with::<ATOMIC, Rule>(input, stack)?;
        input = next;

        Ok((
            input,
            Self {
                first,
                second,
                _phantom: PhantomData,
            },
        ))
    }
}
impl<
        'i,
        R: RuleType,
        T1: TypedNode<'i, R>,
        T2: TypedNode<'i, R>,
        IGNORED: NeverFailedTypedNode<'i, R>,
    > Debug for Seq<'i, R, T1, T2, IGNORED>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Seq")
            .field("first", &self.first)
            .field("second", &self.second)
            .finish()
    }
}

/// Match either of two expressions
pub enum Choice<'i, R: RuleType, T1: TypedNode<'i, R>, T2: TypedNode<'i, R>> {
    /// Matched first expression.
    First(T1, PhantomData<&'i R>),
    /// Matched second expression.
    Second(T2, PhantomData<&'i R>),
}
impl<'i, R: RuleType, T1: TypedNode<'i, R>, T2: TypedNode<'i, R>> Choice<'i, R, T1, T2> {
    /// Get the first case if exists.
    #[inline]
    pub fn get_first(&self) -> Option<&T1> {
        match self {
            Self::First(res, _) => Some(res),
            Self::Second(_, _) => None,
        }
    }
    /// Get the second case if exists.
    #[inline]
    pub fn get_second(&self) -> Option<&T2> {
        match self {
            Self::First(_, _) => None,
            Self::Second(res, _) => Some(res),
        }
    }
}
impl<'i, R: RuleType, T1: TypedNode<'i, R>, T2: TypedNode<'i, R>> TypedNode<'i, R>
    for Choice<'i, R, T1, T2>
{
    #[inline]
    fn try_parse_with<const ATOMIC: bool, Rule: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        match T1::try_parse_with::<ATOMIC, Rule>(input, stack) {
            Ok((input, first)) => Ok((input, Self::First(first, PhantomData))),
            Err(first) => match T2::try_parse_with::<ATOMIC, Rule>(input, stack) {
                Ok((input, second)) => Ok((input, Self::Second(second, PhantomData))),
                Err(second) => Err(first.merge(second)),
            },
        }
    }
}
impl<'i, R: RuleType, T1: TypedNode<'i, R>, T2: TypedNode<'i, R>> Debug for Choice<'i, R, T1, T2> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::First(first, _) => f.debug_tuple("First").field(first).finish(),
            Self::Second(second, _) => f.debug_tuple("Second").field(second).finish(),
        }
    }
}

/// Repeatably match `T`.
pub struct Rep<'i, R: RuleType, T: TypedNode<'i, R>, IGNORED: NeverFailedTypedNode<'i, R>> {
    /// Matched pairs.
    pub content: Vec<T>,
    _phantom: PhantomData<(&'i R, &'i IGNORED)>,
}
impl<'i, R: RuleType, T: TypedNode<'i, R>, IGNORED: NeverFailedTypedNode<'i, R>> TypedNode<'i, R>
    for Rep<'i, R, T, IGNORED>
{
    #[inline]
    fn try_parse_with<const ATOMIC: bool, Rule: RuleWrapper<R>>(
        mut input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        let mut vec = Vec::<T>::new();

        {
            let mut i: usize = 0;
            loop {
                if i != 0 {
                    let (next, _) = IGNORED::parse_with::<ATOMIC, Rule>(input, stack);
                    input = next;
                }
                match T::try_parse_with::<ATOMIC, Rule>(input, stack) {
                    Ok((next, elem)) => {
                        input = next;
                        vec.push(elem);
                    }
                    Err(_err) => {
                        break;
                    }
                }
                i += 1;
                if i > 1024 {
                    return Err(Tracker::RepeatTooManyTimes(input));
                }
            }
        }
        Ok((
            input,
            Self {
                content: vec,
                _phantom: PhantomData,
            },
        ))
    }
}
impl<'i, R: RuleType, T: TypedNode<'i, R>, IGNORED: NeverFailedTypedNode<'i, R>> Debug
    for Rep<'i, R, T, IGNORED>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rep")
            .field("content", &self.content)
            .finish()
    }
}

/// Drops the top of the stack.
pub struct DROP<'i> {
    _phantom: PhantomData<&'i str>,
}

impl<'i, R: RuleType> TypedNode<'i, R> for DROP<'i> {
    #[inline]
    fn try_parse_with<const _A: bool, Rule: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        match stack.pop() {
            Some(_) => Ok((
                input,
                Self {
                    _phantom: PhantomData,
                },
            )),
            None => Err(Tracker::EmptyStack(input)),
        }
    }
}
impl<'i> Debug for DROP<'i> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DROP").finish()
    }
}

/// Match and pop the top of the stack.
pub struct POP<'i> {
    /// Matched span.
    pub span: Span<'i>,
}

impl<'i> From<Span<'i>> for POP<'i> {
    fn from(span: Span<'i>) -> Self {
        Self { span }
    }
}
impl<'i, R: RuleType> TypedNode<'i, R> for POP<'i> {
    #[inline]
    fn try_parse_with<const _A: bool, Rule: RuleWrapper<R>>(
        mut input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        match stack.pop() {
            Some(span) => match input.match_string(span.as_str()) {
                true => Ok((input, Self::from(span))),
                false => Err(Tracker::new(input)),
            },
            None => Err(Tracker::EmptyStack(input)),
        }
    }
}
impl<'i> Debug for POP<'i> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("POP").finish()
    }
}

/// Match and pop all elements in the stack.
#[allow(non_camel_case_types)]
#[derive(Debug)]
pub struct POP_ALL<'i> {
    /// Matched span.
    pub span: Span<'i>,
}

impl<'i> From<Span<'i>> for POP_ALL<'i> {
    fn from(span: Span<'i>) -> Self {
        Self { span }
    }
}
impl<'i, R: RuleType> TypedNode<'i, R> for POP_ALL<'i> {
    #[inline]
    fn try_parse_with<const ATOMIC: bool, Rule: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        let (input, res) = PEEK_ALL::try_parse_with::<ATOMIC, Rule>(input, stack)?;
        while let Some(_) = stack.pop() {}
        Ok((input, Self::from(res.span)))
    }
}

/// Boxed node for `T`.
pub struct Box<'i, R: RuleType, T: TypedNode<'i, R>> {
    /// Boxed content.
    pub content: ::alloc::boxed::Box<T>,
    _phantom: PhantomData<&'i R>,
}
impl<'i, R: RuleType, T: TypedNode<'i, R>> TypedNode<'i, R> for Box<'i, R, T> {
    #[inline]
    fn try_parse_with<const ATOMIC: bool, Rule: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        let (input, res) = T::try_parse_with::<ATOMIC, Rule>(input, stack)?;
        Ok((
            input,
            Self {
                content: ::alloc::boxed::Box::new(res),
                _phantom: PhantomData,
            },
        ))
    }
}
impl<'i, R: RuleType, T: TypedNode<'i, R>> Debug for Box<'i, R, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.content.fmt(f)
    }
}

/// Restore stack state on error.
pub struct Restorable<'i, R: RuleType, T: TypedNode<'i, R>> {
    /// Matched content.
    pub content: T,
    _phantom: PhantomData<&'i R>,
}
impl<'i, R: RuleType, T: TypedNode<'i, R>> From<T> for Restorable<'i, R, T> {
    fn from(content: T) -> Self {
        Self {
            content,
            _phantom: PhantomData,
        }
    }
}
impl<'i, R: RuleType, T: TypedNode<'i, R>> TypedNode<'i, R> for Restorable<'i, R, T> {
    fn try_parse_with<const ATOMIC: bool, Rule: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        stack.snapshot();
        match T::try_parse_with::<ATOMIC, Rule>(input, stack) {
            Ok((input, res)) => {
                stack.clear_snapshot();
                Ok((input, Self::from(res)))
            }
            Err(err) => {
                stack.restore();
                Err(err)
            }
        }
    }
}
impl<'i, R: RuleType, T: TypedNode<'i, R>> Debug for Restorable<'i, R, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.content.fmt(f)
    }
}

/// Always fail.
pub struct AlwaysFail<'i> {
    _phantom: PhantomData<&'i ()>,
}
/// A trait that only `AlwaysFail` implements.
pub trait AlwaysFailed: Debug {}
impl<'i> AlwaysFailed for AlwaysFail<'i> {}
impl<'i, R: RuleType, T: AlwaysFailed> TypedNode<'i, R> for T {
    #[inline]
    fn try_parse_with<const ATOMIC: bool, Rule: RuleWrapper<R>>(
        input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        Err(Tracker::new(input))
    }
}
impl<'i> Debug for AlwaysFail<'i> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AlwaysFail").finish()
    }
}

/// Start point of a rule.
///
/// Force inner tokens to be atomic.
///
/// See [`Rule`] and [`NonAtomicRule`].
pub struct AtomicRule<
    'i,
    R: RuleType,
    T: TypedNode<'i, R>,
    RULE: RuleWrapper<R>,
    _EOI: RuleWrapper<R>,
> {
    /// Matched content.
    pub content: T,
    _phantom: PhantomData<(&'i R, &'i RULE, &'i _EOI)>,
}

impl<'i, R: RuleType, T: TypedNode<'i, R>, RULE: RuleWrapper<R>, _EOI: RuleWrapper<R>> From<T>
    for AtomicRule<'i, R, T, RULE, _EOI>
{
    fn from(content: T) -> Self {
        Self {
            content,
            _phantom: PhantomData,
        }
    }
}
impl<'i, R: RuleType, T: TypedNode<'i, R>, RULE: RuleWrapper<R>, _EOI: RuleWrapper<R>>
    TypedNode<'i, R> for AtomicRule<'i, R, T, RULE, _EOI>
{
    #[inline]
    fn try_parse_with<const ATOMIC: bool, _Rule: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        match T::try_parse_with::<true, RULE>(input, stack) {
            Ok((input, res)) => Ok((input, Self::from(res))),
            Err(err) => Err(err.nest(RULE::RULE, input)),
        }
    }
}
impl<'i, R: RuleType, T: TypedNode<'i, R>, RULE: RuleWrapper<R>, _EOI: RuleWrapper<R>>
    ParsableTypedNode<'i, R> for AtomicRule<'i, R, T, RULE, _EOI>
{
    fn parse(input: &'i str) -> Result<Self, Error<R>> {
        parse_without_ignore::<R, RULE, _EOI, Self>(input)
    }
    fn parse_partial(input: &'i str) -> Result<(Position<'i>, Self), Error<R>> {
        parse_partial::<R, RULE, Self>(input)
    }
}
impl<'i, R: RuleType, T: TypedNode<'i, R>, RULE: RuleWrapper<R>, _EOI: RuleWrapper<R>> Debug
    for AtomicRule<'i, R, T, RULE, _EOI>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AtomicRule")
            .field("content", &self.content)
            .finish()
    }
}

/// Start point of a rule.
///
/// Force inner tokens to be not atomic.
///
/// See [`Rule`] and [`AtomicRule`].
pub struct NonAtomicRule<
    'i,
    R: RuleType,
    T: TypedNode<'i, R>,
    RULE: RuleWrapper<R>,
    _EOI: RuleWrapper<R>,
    IGNORED: NeverFailedTypedNode<'i, R>,
> {
    /// Matched content.
    pub content: T,
    _phantom: PhantomData<(&'i R, &'i T, &'i RULE, &'i _EOI, &'i IGNORED)>,
}
impl<
        'i,
        R: RuleType,
        T: TypedNode<'i, R>,
        RULE: RuleWrapper<R>,
        _EOI: RuleWrapper<R>,
        IGNORED: NeverFailedTypedNode<'i, R>,
    > From<T> for NonAtomicRule<'i, R, T, RULE, _EOI, IGNORED>
{
    fn from(content: T) -> Self {
        Self {
            content,
            _phantom: PhantomData,
        }
    }
}
impl<
        'i,
        R: RuleType,
        T: TypedNode<'i, R>,
        RULE: RuleWrapper<R>,
        _EOI: RuleWrapper<R>,
        IGNORED: NeverFailedTypedNode<'i, R>,
    > TypedNode<'i, R> for NonAtomicRule<'i, R, T, RULE, _EOI, IGNORED>
{
    #[inline]
    fn try_parse_with<const ATOMIC: bool, _Rule: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        match T::try_parse_with::<false, RULE>(input, stack) {
            Ok((input, res)) => Ok((input, Self::from(res))),
            Err(err) => Err(err.nest(RULE::RULE, input)),
        }
    }
}
impl<
        'i,
        R: RuleType,
        T: TypedNode<'i, R>,
        RULE: RuleWrapper<R>,
        _EOI: RuleWrapper<R>,
        IGNORED: NeverFailedTypedNode<'i, R>,
    > ParsableTypedNode<'i, R> for NonAtomicRule<'i, R, T, RULE, _EOI, IGNORED>
{
    fn parse(input: &'i str) -> Result<Self, Error<R>> {
        parse::<R, RULE, _EOI, Self, IGNORED>(input)
    }
    fn parse_partial(input: &'i str) -> Result<(Position<'i>, Self), Error<R>> {
        parse_partial::<R, RULE, Self>(input)
    }
}
impl<
        'i,
        R: RuleType,
        T: TypedNode<'i, R>,
        RULE: RuleWrapper<R>,
        _EOI: RuleWrapper<R>,
        IGNORED: NeverFailedTypedNode<'i, R>,
    > Debug for NonAtomicRule<'i, R, T, RULE, _EOI, IGNORED>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NonAtomicRule")
            .field("content", &self.content)
            .finish()
    }
}

/// Match an expression and push it.
pub struct Push<'i, R: RuleType, T: TypedNode<'i, R>> {
    /// Matched content.
    pub content: T,
    _phantom: PhantomData<(&'i R, &'i T)>,
}
impl<'i, R: RuleType, T: TypedNode<'i, R>> From<T> for Push<'i, R, T> {
    fn from(content: T) -> Self {
        Self {
            content,
            _phantom: PhantomData,
        }
    }
}
impl<'i, R: RuleType, T: TypedNode<'i, R>> TypedNode<'i, R> for Push<'i, R, T> {
    #[inline]
    fn try_parse_with<const ATOMIC: bool, Rule: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        let start = input.clone();
        let (input, content) = T::try_parse_with::<ATOMIC, Rule>(input, stack)?;
        stack.push(start.span(&input));
        Ok((input, Self::from(content)))
    }
}
impl<'i, R: RuleType, T: TypedNode<'i, R>> Debug for Push<'i, R, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Push")
            .field("content", &self.content)
            .finish()
    }
}

/// Match `START`..`END` of the stack.
pub struct PeekSlice2<'i, R: RuleType, const START: i32, const END: i32> {
    _phantom: PhantomData<&'i R>,
}
impl<'i, R: RuleType, const START: i32, const END: i32> TypedNode<'i, R>
    for PeekSlice2<'i, R, START, END>
{
    #[inline]
    fn try_parse_with<const ATOMIC: bool, Rule: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        let spans = stack_slice(input, START, Some(END), stack)?;
        let (input, _) = peek_spans::<R, Rule>(input, spans)?;
        Ok((
            input,
            Self {
                _phantom: PhantomData,
            },
        ))
    }
}
impl<'i, R: RuleType, const START: i32, const END: i32> Debug for PeekSlice2<'i, R, START, END> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PeekSlice2").finish()
    }
}

/// Match `START`..`END` of the stack.
pub struct PeekSlice1<'i, R: RuleType, const START: i32> {
    _phantom: PhantomData<&'i R>,
}
impl<'i, R: RuleType, const START: i32> TypedNode<'i, R> for PeekSlice1<'i, R, START> {
    #[inline]
    fn try_parse_with<const ATOMIC: bool, Rule: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        let spans = stack_slice(input, START, None, stack)?;
        let (input, _) = peek_spans::<R, Rule>(input, spans)?;
        Ok((
            input,
            Self {
                _phantom: PhantomData,
            },
        ))
    }
}
impl<'i, R: RuleType, const START: i32> Debug for PeekSlice1<'i, R, START> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PeekSlice1").finish()
    }
}

/// Start point of a rule.
///
/// Will not change atomicity.
///
/// See [`AtomicRule`] and [`NonAtomicRule`].
pub struct Rule<
    'i,
    R: RuleType,
    RULE: RuleWrapper<R>,
    _EOI: RuleWrapper<R>,
    T: TypedNode<'i, R>,
    IGNORED: NeverFailedTypedNode<'i, R>,
> {
    /// Matched content.
    pub content: T,
    _phantom: PhantomData<(&'i R, &'i RULE, &'i _EOI, &'i IGNORED)>,
}
impl<
        'i,
        R: RuleType,
        RULE: RuleWrapper<R>,
        _EOI: RuleWrapper<R>,
        T: TypedNode<'i, R>,
        IGNORED: NeverFailedTypedNode<'i, R>,
    > TypeWrapper for Rule<'i, R, RULE, _EOI, T, IGNORED>
{
    type Inner = T;
}
impl<
        'i,
        R: RuleType,
        RULE: RuleWrapper<R>,
        _EOI: RuleWrapper<R>,
        T: TypedNode<'i, R>,
        IGNORED: NeverFailedTypedNode<'i, R>,
    > TypedNode<'i, R> for Rule<'i, R, RULE, _EOI, T, IGNORED>
{
    #[inline]
    fn try_parse_with<const ATOMIC: bool, _Rule: RuleWrapper<R>>(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
    ) -> Result<(Position<'i>, Self), Tracker<'i, R>> {
        match T::try_parse_with::<ATOMIC, RULE>(input, stack) {
            Ok((input, res)) => Ok((
                input,
                Self {
                    content: res,
                    _phantom: PhantomData,
                },
            )),
            Err(err) => Err(err.nest(RULE::RULE, input)),
        }
    }
}
impl<
        'i,
        R: RuleType,
        RULE: RuleWrapper<R>,
        _EOI: RuleWrapper<R>,
        T: TypedNode<'i, R>,
        IGNORED: NeverFailedTypedNode<'i, R>,
    > ParsableTypedNode<'i, R> for Rule<'i, R, RULE, _EOI, T, IGNORED>
{
    #[inline]
    fn parse(input: &'i str) -> Result<Self, Error<R>> {
        parse::<R, RULE, _EOI, Self, IGNORED>(input)
    }

    fn parse_partial(input: &'i str) -> Result<(Position<'i>, Self), Error<R>> {
        parse_partial::<R, RULE, Self>(input)
    }
}
impl<
        'i,
        R: RuleType,
        RULE: RuleWrapper<R>,
        _EOI: RuleWrapper<R>,
        T: TypedNode<'i, R>,
        IGNORED: NeverFailedTypedNode<'i, R>,
    > Debug for Rule<'i, R, RULE, _EOI, T, IGNORED>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rule").finish()
    }
}

fn parse<
    'i,
    R: RuleType,
    RULE: RuleWrapper<R>,
    _EOI: RuleWrapper<R>,
    _Self: TypedNode<'i, R>,
    IGNORED: NeverFailedTypedNode<'i, R>,
>(
    input: &'i str,
) -> Result<_Self, Error<R>> {
    let mut stack = Stack::new();
    let (input, res) =
        match _Self::try_parse_with::<false, RULE>(Position::from_start(input), &mut stack) {
            Ok((input, res)) => (input, res),
            Err(e) => return Err(e.collect()),
        };
    let (input, _) = IGNORED::parse_with::<false, _EOI>(input, &mut stack);
    let (_, _) = match EOI::try_parse_with::<false, _EOI>(input, &mut stack) {
        Ok((input, res)) => (input, res),
        Err(e) => return Err(e.collect()),
    };
    Ok(res)
}

fn parse_without_ignore<
    'i,
    R: RuleType,
    RULE: RuleWrapper<R>,
    _EOI: RuleWrapper<R>,
    _Self: TypedNode<'i, R>,
>(
    input: &'i str,
) -> Result<_Self, Error<R>> {
    let mut stack = Stack::new();
    let (input, res) =
        match _Self::try_parse_with::<false, RULE>(Position::from_start(input), &mut stack) {
            Ok((input, res)) => (input, res),
            Err(e) => return Err(e.collect()),
        };
    let (_, _) = match EOI::try_parse_with::<false, _EOI>(input, &mut stack) {
        Ok((input, res)) => (input, res),
        Err(e) => return Err(e.collect()),
    };
    Ok(res)
}

fn parse_partial<'i, R: RuleType, RULE: RuleWrapper<R>, _Self: TypedNode<'i, R>>(
    input: &'i str,
) -> Result<(Position<'i>, _Self), Error<R>> {
    let mut stack = Stack::new();
    match _Self::try_parse_with::<false, RULE>(Position::from_start(input), &mut stack) {
        Ok((input, res)) => Ok((input, res)),
        Err(e) => return Err(e.collect()),
    }
}

/// ASCII Digit. `'0'..'9'`
#[allow(non_camel_case_types)]
pub type ASCII_DIGIT<'i, R> = CharRange<'i, R, '0', '9'>;

/// Non-zero ASCII Digit. `'1'..'9'`
#[allow(non_camel_case_types)]
pub type ASCII_NONZERO_DIGIT<'i, R> = CharRange<'i, R, '1', '9'>;

/// Binary ASCII Digit. `'0'..'1'`
#[allow(non_camel_case_types)]
pub type ASCII_BIN_DIGIT<'i, R> = CharRange<'i, R, '0', '1'>;

/// Octal ASCII Digit. `'0'..'7'`
#[allow(non_camel_case_types)]
pub type ASCII_OCT_DIGIT<'i, R> = CharRange<'i, R, '0', '7'>;

/// Hexadecimal ASCII Digit. `'0'..'9' | 'a'..'f' | 'A'..'F'`
#[allow(non_camel_case_types)]
pub type ASCII_HEX_DIGIT<'i, R> = Choice<
    'i,
    R,
    ASCII_DIGIT<'i, R>,
    Choice<'i, R, CharRange<'i, R, 'a', 'f'>, CharRange<'i, R, 'A', 'F'>>,
>;

/// Lower case ASCII alphabet.
#[allow(non_camel_case_types)]
pub type ASCII_ALPHA_LOWER<'i, R> = CharRange<'i, R, 'a', 'z'>;

/// Upper case ASCII alphabet.
#[allow(non_camel_case_types)]
pub type ASCII_ALPHA_UPPER<'i, R> = CharRange<'i, R, 'A', 'Z'>;

/// ASCII alphabet.
#[allow(non_camel_case_types)]
pub type ASCII_ALPHA<'i, R> = Choice<'i, R, ASCII_ALPHA_LOWER<'i, R>, ASCII_ALPHA_UPPER<'i, R>>;

/// ASCII alphabet or digit.
#[allow(non_camel_case_types)]
pub type ASCII_ALPHANUMERIC<'i, R> = Choice<'i, R, ASCII_ALPHA<'i, R>, ASCII_DIGIT<'i, R>>;

/// ASCII alphabet.
#[allow(non_camel_case_types)]
pub type ASCII<'i, R> = CharRange<'i, R, '\x00', '\x7f'>;

/// Match char by
pub fn match_char_by(position: &mut Position<'_>, pred: impl FnOnce(char) -> bool) -> Option<char> {
    let mut res = None;
    position.match_char_by(|c| {
        let matched = pred(c);
        if matched {
            res = Some(c);
        }
        matched
    });
    res
}

#[cfg(test)]
mod tests {

    use super::super::Storage;

    use super::*;

    macro_rules! make_rules {
        ($($ids:ident,)*) => {
            #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
            enum Rule {
                $($ids),*
            }
            mod rule_wrappers {
                $(
                    pub struct $ids {}
                    impl super::RuleWrapper<super::Rule> for $ids {
                        const RULE:super::Rule = super::Rule::$ids;
                    }
                )*
            }
        };
    }

    make_rules! {
        Foo,
        RepFoo,
        WHITESPACE,
        COMMENT,
        EOI,
    }

    struct Foo;
    impl StringWrapper for Foo {
        const CONTENT: &'static str = "foo";
    }
    impl RuleWrapper<Rule> for Foo {
        const RULE: Rule = Rule::Foo;
    }

    type WHITESPACE<'i> = AtomicRule<
        'i,
        Rule,
        CharRange<'i, Rule, ' ', ' '>,
        rule_wrappers::WHITESPACE,
        rule_wrappers::EOI,
    >;
    type COMMENT<'i> = AtomicRule<
        'i,
        Rule,
        CharRange<'i, Rule, '\t', '\t'>,
        rule_wrappers::COMMENT,
        rule_wrappers::EOI,
    >;
    type StrFoo<'i> = super::Rule<
        'i,
        Rule,
        rule_wrappers::Foo,
        rule_wrappers::EOI,
        Str<'i, Rule, Foo>,
        Ignore<'i>,
    >;
    #[test]
    fn string() {
        assert_eq!(<StrFoo<'_> as TypeWrapper>::Inner::CONTENT, Foo::CONTENT);
        let s = StrFoo::parse("foo").unwrap();
        assert_eq!(s.content.get_content(), "foo");
    }
    #[test]
    fn range() {
        WHITESPACE::parse(" ").unwrap();
        COMMENT::parse("\t").unwrap();
    }
    type Ignore<'i> = Ign<'i, Rule, COMMENT<'i>, WHITESPACE<'i>>;
    #[test]
    fn ignore() {
        super::Rule::<Rule, rule_wrappers::RepFoo, rule_wrappers::EOI, Ignore<'_>, Ignore<'_>>::parse(
            " \t  ",
        )
        .unwrap();
    }

    type R<'i> = super::Rule<
        'i,
        Rule,
        rule_wrappers::RepFoo,
        rule_wrappers::EOI,
        Rep<'i, Rule, Str<'i, Rule, Foo>, Ignore<'i>>,
        Ignore<'i>,
    >;
    #[test]
    fn repetition() {
        R::parse("foofoofoo").unwrap();
        R::parse("foo foo foo").unwrap();
        R::parse("foo foo\tfoo").unwrap();
    }
}
