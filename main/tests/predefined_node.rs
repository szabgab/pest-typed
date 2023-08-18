// pest-typed. A statically typed version of pest.
// Copyright (c) 2023 黄博奕
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

extern crate alloc;

use pest_typed::predefined_node::*;

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;
    use pest_typed::{
        atomic_rule, compound_atomic_rule, non_atomic_rule, normal_rule, rule_eoi, silent_rule,
        BoundWrapper, ParsableTypedNode, RuleWrapper, Storage, StringWrapper, TypeWrapper,
    };

    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    enum Rule {
        Foo,
        RepFoo,
        WHITESPACE,
        COMMENT,
        EOI,
    }

    #[derive(Clone, PartialEq)]
    pub struct Foo;
    impl StringWrapper for Foo {
        const CONTENT: &'static str = "foo";
    }
    impl RuleWrapper<Rule> for Foo {
        const RULE: Rule = Rule::Foo;
        type Rule = Rule;
    }

    atomic_rule!(
        WHITESPACE,
        "White space.",
        Rule,
        Rule::WHITESPACE,
        CharRange::<' ', ' '>
    );
    compound_atomic_rule!(
        COMMENT,
        "Comment",
        Rule,
        Rule::COMMENT,
        CharRange::<'\t', '\t'>
    );
    normal_rule!(
        StrFoo,
        "String \"Foo\"",
        Rule,
        Rule::Foo,
        Str::<Foo>,
        Ignore::<'i>
    );
    rule_eoi!(EOI, Rule);
    #[test]
    fn string() {
        assert_eq!(<StrFoo<'_> as TypeWrapper>::Inner::CONTENT, Foo::CONTENT);
        let s = StrFoo::parse("foo").unwrap();
        assert_eq!(s.content.get_content(), "foo");
        assert_eq!(
            format!("{:?}", s),
            r#"Rule { name: "StrFoo", content: Str, span: Span { str: "foo", start: 0, end: 3 } }"#
        )
    }
    #[test]
    fn range() {
        let whitespace = WHITESPACE::parse(" ").unwrap();
        assert_eq!(
            format!("{:?}", whitespace),
            "Rule { name: \"WHITESPACE\", span: Span { str: \" \", start: 0, end: 1 } }"
        );
        let comment = COMMENT::parse("\t").unwrap();
        assert_eq!(
            format!("{:?}", comment),
            "Rule { name: \"COMMENT\", content: CharRange { content: '\\t' }, span: Span { str: \"\\t\", start: 0, end: 1 } }"
        );
    }
    type Ignore<'i> = Skipped<COMMENT<'i>, WHITESPACE<'i>>;
    #[test]
    fn ignore() {
        silent_rule!(
            tmp,
            "Temporary rule.",
            Rule,
            Rule::RepFoo,
            Ignore<'i>,
            Ignore<'i>
        );
        tmp::parse(" \t  ").unwrap();
    }

    type REP<'i> = Rep<Str<Foo>, Ignore<'i>>;
    non_atomic_rule!(
        R,
        "Repetion of [StrFoo].",
        Rule,
        Rule::RepFoo,
        REP<'i>,
        Ignore<'i>
    );
    #[test]
    fn repetition() {
        let rep1 = R::parse("foofoofoo").unwrap();
        let rep2 = R::parse("foo foo foo").unwrap();
        let rep3 = R::parse("foo foo\tfoo").unwrap();
        assert_ne!(rep1, rep2);
        assert_ne!(rep1, rep3);
        let format = |rep: &R<'_>| -> String {
            rep.iter()
                .map(|e| e.get_content())
                .collect::<Vec<_>>()
                .concat()
        };
        assert_eq!(format(&rep1), format(&rep2));
        assert_eq!(format(&rep1), format(&rep3));
        assert_eq!(REP::MIN, 0);
    }
}
