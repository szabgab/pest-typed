// pest-typed. A statically typed version of pest.
// Copyright (c) 2023 黄博奕
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Predefined tree nodes generics.
//! The generator may use this for convenience.
//! Normally you don't need to reference this module by yourself.

mod repetition;

use super::{parser_state::constrain_idxs, position::Position, Stack};
use super::{
    span::Span,
    tracker::Tracker,
    typed_node::NeverFailedTypedNode,
    wrapper::{StringArrayWrapper, StringWrapper},
    RuleType, TypedNode,
};
use core::ops::{Deref, DerefMut};
use core::{fmt::Debug, marker::PhantomData};
use custom_debug_derive::Debug as Dbg;
pub use repetition::{AtomicRep, Rep, RepMin, RepMinMax, RepOnce};

/// Match given string case sensitively.
///
/// The `CONTENT` on the type (by [`StringWrapper`]) is the original string to match.
///
/// See [`Insens`] for case-insensitive matching.
#[derive(Clone, Dbg, Hash, PartialEq, Eq)]
pub struct Str<T: StringWrapper + 'static> {
    #[debug(skip)]
    _phantom: PhantomData<&'static T>,
}
impl<T: StringWrapper> StringWrapper for Str<T> {
    const CONTENT: &'static str = T::CONTENT;
}
impl<T: StringWrapper> From<()> for Str<T> {
    fn from(_value: ()) -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}
impl<'i, R: RuleType, T: StringWrapper> TypedNode<'i, R> for Str<T> {
    fn try_parse_with(
        mut input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
        _tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        if input.match_string(Self::CONTENT) {
            Some((input, Self::from(())))
        } else {
            None
        }
    }
}

/// Match given string case insensitively.
///
/// - The field `content` is the matched string.
/// - The `CONTENT` on the type (by [`StringWrapper`]) is the original string to match, and it may differ from `content` in case.
///   
///   For example, A `^"x"` may match `"X"`, and in the parsing result, `self.content` is `"X"`, while `Self::CONTENT` is still `"x"`.    
///
/// See [`Str`] for case-sensitive matching.
#[derive(Clone, Dbg, Hash, PartialEq, Eq)]
pub struct Insens<'i, T: StringWrapper> {
    /// Matched content.
    pub content: &'i str,
    #[debug(skip)]
    _phantom: PhantomData<&'i T>,
}
impl<'i, T: StringWrapper> StringWrapper for Insens<'i, T> {
    const CONTENT: &'static str = T::CONTENT;
}
impl<'i, T: StringWrapper> From<&'i str> for Insens<'i, T> {
    fn from(content: &'i str) -> Self {
        Self {
            content,
            _phantom: PhantomData,
        }
    }
}
impl<'i, R: RuleType, T: StringWrapper> TypedNode<'i, R> for Insens<'i, T> {
    fn try_parse_with(
        mut input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
        _tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        let start = input;
        if input.match_insensitive(Self::CONTENT) {
            let span = start.span(&input);
            Some((input, Self::from(span.as_str())))
        } else {
            None
        }
    }
}

/// Skips until one of the given strings.
#[derive(Clone, Dbg, Hash, PartialEq, Eq)]
pub struct Skip<'i, Strings: StringArrayWrapper> {
    /// Skipped span.
    pub span: Span<'i>,
    #[debug(skip)]
    _phantom: PhantomData<&'i Strings>,
}
impl<'i, Strings: StringArrayWrapper> StringArrayWrapper for Skip<'i, Strings> {
    const CONTENT: &'static [&'static str] = Strings::CONTENT;
}
impl<'i, Strings: StringArrayWrapper> From<Span<'i>> for Skip<'i, Strings> {
    fn from(span: Span<'i>) -> Self {
        Self {
            span,
            _phantom: PhantomData,
        }
    }
}
impl<'i, R: RuleType, Strings: StringArrayWrapper> TypedNode<'i, R> for Skip<'i, Strings> {
    fn try_parse_with(
        mut input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
        _tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        let start = input;
        match input.skip_until(Strings::CONTENT) {
            true => {
                let span = start.span(&input);
                Some((input, Self::from(span)))
            }
            false => None,
        }
    }
}

/// Skip `n` characters if there are.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct SkipChar<'i, const N: usize> {
    /// Skipped span.
    pub span: Span<'i>,
}
impl<'i, R: RuleType, const N: usize> TypedNode<'i, R> for SkipChar<'i, N> {
    fn try_parse_with(
        mut input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
        _tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        let start = input;
        match input.skip(N) {
            true => {
                let span = start.span(&input);
                Some((input, Self { span }))
            }
            false => None,
        }
    }
}

/// Match a character in the range `[MIN, MAX]`.
/// Inclusively both below and above.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct CharRange<const MIN: char, const MAX: char> {
    /// Matched character.
    pub content: char,
}
impl<'i, R: RuleType, const MIN: char, const MAX: char> TypedNode<'i, R> for CharRange<MIN, MAX> {
    fn try_parse_with(
        mut input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
        _tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        let start = input;
        match input.match_range(MIN..MAX) {
            true => {
                let span = start.span(&input);
                let content = span.as_str().chars().next().unwrap();
                Some((input, Self { content }))
            }
            false => None,
        }
    }
}

/// Try to create stack slice.
#[inline]
fn stack_slice<'i, 's, R: RuleType>(
    input: Position<'i>,
    start: i32,
    end: Option<i32>,
    stack: &'s Stack<Span<'i>>,
    tracker: &mut Tracker<'i, R>,
) -> Option<core::slice::Iter<'s, Span<'i>>> {
    let range = match constrain_idxs(start, end, stack.len()) {
        Some(range) => range,
        None => {
            tracker.out_of_bound(input, start, end);
            return None;
        }
    };
    // return true if an empty sequence is requested
    if range.end <= range.start {
        return Some(core::slice::Iter::default());
    }
    Some(stack[range].iter())
}

/// Match a part of the stack without popping.
/// Will match (consume) input.
#[inline]
fn peek_spans<'s, 'i: 's, R: RuleType>(
    input: Position<'i>,
    iter: impl Iterator<Item = &'s Span<'i>>,
    _tracker: &mut Tracker<'i, R>,
) -> Option<(Position<'i>, Span<'i>)> {
    let mut matching_pos = input;
    for span in iter {
        match matching_pos.match_string(span.as_str()) {
            true => (),
            false => {
                return None;
            }
        }
    }
    Some((matching_pos, input.span(&matching_pos)))
}

/// Positive predicate.
///
/// Peeked expressions will not occur in Pair/Pairs API.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Positive<N> {
    /// Peeked content.
    pub content: N,
}
impl<N> From<N> for Positive<N> {
    fn from(content: N) -> Self {
        Self { content }
    }
}
impl<N> Deref for Positive<N> {
    type Target = N;
    fn deref(&self) -> &Self::Target {
        &self.content
    }
}
impl<N> DerefMut for Positive<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.content
    }
}
impl<'i, R: RuleType, N: TypedNode<'i, R>> TypedNode<'i, R> for Positive<N> {
    fn try_parse_with(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
        tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        tracker.positive_during(|tracker| {
            stack.snapshot();
            match N::try_parse_with(input, stack, tracker) {
                Some((_, content)) => {
                    stack.restore();
                    Some((input, Self::from(content)))
                }
                None => {
                    stack.restore();
                    None
                }
            }
        })
    }
}

/// Negative predicate.
///
/// Will not contain anything.
#[derive(Clone, Dbg, Hash, PartialEq, Eq)]
pub struct Negative<T> {
    #[debug(skip)]
    _phantom: PhantomData<T>,
}
impl<T> From<()> for Negative<T> {
    fn from(_value: ()) -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}
impl<'i, R: RuleType, T: TypedNode<'i, R>> TypedNode<'i, R> for Negative<T> {
    fn try_parse_with(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
        tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        tracker.negative_during(|tracker| {
            stack.snapshot();
            match T::try_parse_with(input, stack, tracker) {
                Some(_) => {
                    stack.restore();
                    None
                }
                None => {
                    stack.restore();
                    Some((input, Self::from(())))
                }
            }
        })
    }
}

/// Match any character.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ANY {
    /// Matched character.
    pub content: char,
}
impl<'i, R: RuleType> TypedNode<'i, R> for ANY {
    #[inline]
    fn try_parse_with(
        mut input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
        _tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        let mut c: char = ' ';
        let matcher = |ch| {
            c = ch;
            true
        };
        match input.match_char_by(matcher) {
            true => Some((input, Self { content: c })),
            false => None,
        }
    }
}

/// Match the start of input.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct SOI;
impl<'i, R: RuleType> TypedNode<'i, R> for SOI {
    #[inline]
    fn try_parse_with(
        input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
        _tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        if input.at_start() {
            Some((input, Self))
        } else {
            None
        }
    }
}

/// Match the end of input.
///
/// [`EOI`] will record its rule if not matched.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct EOI;
impl<'i, R: RuleType> TypedNode<'i, R> for EOI {
    #[inline]
    fn try_parse_with(
        input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
        _tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        if input.at_end() {
            Some((input, Self))
        } else {
            None
        }
    }
}

/// Type of eol.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum NewLineType {
    /// `\r\n`
    CRLF,
    /// `\n`
    LF,
    /// `\r`
    CR,
}

/// Match a new line character.
/// A built-in rule. Equivalent to `"\r\n" | "\n" | "\r"`.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct NEWLINE {
    /// Type of matched character.
    pub content: NewLineType,
}
impl<'i, R: RuleType> TypedNode<'i, R> for NEWLINE {
    #[inline]
    fn try_parse_with(
        mut input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
        _tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        let (input, t) = if input.match_string("\r\n") {
            (input, NewLineType::CRLF)
        } else if input.match_string("\n") {
            (input, NewLineType::LF)
        } else if input.match_string("\r") {
            (input, NewLineType::CR)
        } else {
            return None;
        };
        Some((input, Self { content: t }))
    }
}

/// Peek all spans in stack reversely.
/// Will consume input.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct PEEK_ALL<'i> {
    /// Pair span.
    pub span: Span<'i>,
}
impl<'i, R: RuleType> TypedNode<'i, R> for PEEK_ALL<'i> {
    #[inline]
    fn try_parse_with(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
        tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        let spans = stack[0..stack.len()].iter().rev();
        let (input, span) = peek_spans::<R>(input, spans, tracker)?;
        Some((input, Self { span }))
    }
}

/// Peek top span in stack.
/// Will consume input.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
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
    fn try_parse_with(
        mut input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
        tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        let start = input;
        match stack.peek() {
            Some(string) => match input.match_string(string.as_str()) {
                true => Some((input, Self::from(start.span(&input)))),
                false => None,
            },
            None => {
                tracker.empty_stack(input);
                None
            }
        }
    }
}

/// Skip comments (by rule `COMMENT`) or white spaces (by rule `WHITESPACE`) if there is any.
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Skipped<T, Skip, const SKIP: usize> {
    /// Skipped content.
    pub skipped: [Skip; SKIP],
    /// Matched content.
    pub matched: T,
}
impl<T: Debug, Skip: Debug, const SKIP: usize> Debug for Skipped<T, Skip, SKIP> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if SKIP > 0 {
            f.debug_struct("Skipped")
                .field("skipped", &self.skipped)
                .field("matched", &self.matched)
                .finish()
        } else {
            Debug::fmt(&self.matched, f)
        }
    }
}

/// Drop the top of the stack.
///
/// Fail if there is no span in the stack.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct DROP;
impl<'i, R: RuleType> TypedNode<'i, R> for DROP {
    #[inline]
    fn try_parse_with(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
        tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        match stack.pop() {
            Some(_) => Some((input, Self)),
            None => {
                tracker.empty_stack(input);
                None
            }
        }
    }
}

/// Match and pop the top span of the stack.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
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
    fn try_parse_with(
        mut input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
        tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        match stack.pop() {
            Some(span) => match input.match_string(span.as_str()) {
                true => Some((input, Self::from(span))),
                false => None,
            },
            None => {
                tracker.empty_stack(input);
                None
            }
        }
    }
}

/// Match and pop all spans in the stack in top-to-bottom-order.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
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
    fn try_parse_with(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
        tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        let (input, res) = PEEK_ALL::try_parse_with(input, stack, tracker)?;
        while stack.pop().is_some() {}
        Some((input, Self::from(res.span)))
    }
}

/// Always fail.
#[derive(Clone, Dbg, Hash, PartialEq, Eq)]
pub struct AlwaysFail<'i>(#[debug(skip)] PhantomData<&'i char>);
impl<'i> Default for AlwaysFail<'i> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
impl<'i, R: RuleType> TypedNode<'i, R> for AlwaysFail<'i> {
    #[inline]
    fn try_parse_with(
        _input: Position<'i>,
        _stack: &mut Stack<Span<'i>>,
        _tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        None
    }
}

/// Empty.
#[derive(Clone, Dbg, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Empty<'i>(#[debug(skip)] PhantomData<&'i char>);
impl<'i> Default for Empty<'i> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
impl<'i, R: RuleType> NeverFailedTypedNode<'i, R> for Empty<'i> {
    #[inline]
    fn parse_with(input: Position<'i>, _stack: &mut Stack<Span<'i>>) -> (Position<'i>, Self) {
        (input, Self::default())
    }
}
impl<'i, R: RuleType> TypedNode<'i, R> for Empty<'i> {
    #[inline]
    fn try_parse_with(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
        _tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        Some(<Self as NeverFailedTypedNode<'i, R>>::parse_with(
            input, stack,
        ))
    }
}

/// Match an expression and push it to the [Stack].
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Push<T> {
    /// Matched content.
    pub content: T,
}
impl<T> From<T> for Push<T> {
    fn from(content: T) -> Self {
        Self { content }
    }
}
impl<'i, R: RuleType, T: TypedNode<'i, R>> TypedNode<'i, R> for Push<T> {
    #[inline]
    fn try_parse_with(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
        tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        let start = input;
        let (input, content) = T::try_parse_with(input, stack, tracker)?;
        stack.push(start.span(&input));
        Some((input, Self::from(content)))
    }
}
impl<T> Deref for Push<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.content
    }
}
impl<T> DerefMut for Push<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.content
    }
}

/// Match `[START..END]` in top-to-bottom order of the stack.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct PeekSlice2<const START: i32, const END: i32>;
impl<'i, R: RuleType, const START: i32, const END: i32> TypedNode<'i, R>
    for PeekSlice2<START, END>
{
    #[inline]
    fn try_parse_with(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
        tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        let spans = stack_slice(input, START, Some(END), stack, tracker)?;
        let (input, _) = peek_spans::<R>(input, spans, tracker)?;
        Some((input, Self))
    }
}

/// Match `[START..]` in top-to-bottom order of the stack.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct PeekSlice1<const START: i32>;
impl<'i, R: RuleType, const START: i32> TypedNode<'i, R> for PeekSlice1<START> {
    #[inline]
    fn try_parse_with(
        input: Position<'i>,
        stack: &mut Stack<Span<'i>>,
        tracker: &mut Tracker<'i, R>,
    ) -> Option<(Position<'i>, Self)> {
        let spans = stack_slice(input, START, None, stack, tracker)?;
        let (input, _) = peek_spans::<R>(input, spans, tracker)?;
        Some((input, Self))
    }
}

/// ASCII Digit. `'0'..'9'`
#[allow(non_camel_case_types)]
pub type ASCII_DIGIT = CharRange<'0', '9'>;

/// Non-zero ASCII Digit. `'1'..'9'`
#[allow(non_camel_case_types)]
pub type ASCII_NONZERO_DIGIT = CharRange<'1', '9'>;

/// Binary ASCII Digit. `'0'..'1'`
#[allow(non_camel_case_types)]
pub type ASCII_BIN_DIGIT = CharRange<'0', '1'>;

/// Octal ASCII Digit. `'0'..'7'`
#[allow(non_camel_case_types)]
pub type ASCII_OCT_DIGIT = CharRange<'0', '7'>;

use crate::choices::{Choice2, Choice3};
/// Hexadecimal ASCII Digit. `'0'..'9' | 'a'..'f' | 'A'..'F'`
#[allow(non_camel_case_types)]
pub type ASCII_HEX_DIGIT = Choice3<ASCII_DIGIT, CharRange<'a', 'f'>, CharRange<'A', 'F'>>;

/// Lower case ASCII alphabet.
#[allow(non_camel_case_types)]
pub type ASCII_ALPHA_LOWER = CharRange<'a', 'z'>;

/// Upper case ASCII alphabet.
#[allow(non_camel_case_types)]
pub type ASCII_ALPHA_UPPER = CharRange<'A', 'Z'>;

/// ASCII alphabet.
#[allow(non_camel_case_types)]
pub type ASCII_ALPHA = Choice2<ASCII_ALPHA_LOWER, ASCII_ALPHA_UPPER>;

/// ASCII alphabet or digit.
#[allow(non_camel_case_types)]
pub type ASCII_ALPHANUMERIC = Choice2<ASCII_ALPHA, ASCII_DIGIT>;

/// ASCII alphabet.
#[allow(non_camel_case_types)]
pub type ASCII = CharRange<'\x00', '\x7f'>;

/// Match char by a predicate.
///
/// Return Some(char) if matched.
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

/// Restore on error.
#[inline]
pub fn restore_on_none<'i, T>(
    stack: &mut Stack<Span<'i>>,
    f: impl FnOnce(&mut Stack<Span<'i>>) -> Option<T>,
) -> Option<T> {
    stack.snapshot();
    let res = f(stack);
    match res.as_ref() {
        Some(_) => stack.clear_snapshot(),
        None => stack.restore(),
    }
    res
}
