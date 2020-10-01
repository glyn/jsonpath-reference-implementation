/*
 * Copyright 2020 VMware, Inc.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use crate::matchers;
use crate::path;
use crate::pest::Parser;
use std::iter;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct PathParser;

pub fn parse<'a>(selector: &'a str) -> Result<Box<dyn path::Path + 'a>, String> {
    let selector_rule = PathParser::parse(Rule::selector, selector)
        .map_err(|e| format!("{}", e))?
        .next()
        .unwrap();

    let mut ms: Vec<Box<&dyn matchers::Matcher>> = Vec::new();
    for r in selector_rule.into_inner() {
        match r.as_rule() {
            Rule::rootSelector => ms.push(Box::new(&matchers::RootSelector {})),

            Rule::matcher => {
                for m in parse_matcher(r) {
                    ms.push(Box::new(m))
                }
            }

            _ => println!("r={:?}", r),
        }
    }

    Ok(Box::new(path::new(ms)))
}

/// An iterator over matcher selection results.
type Iter<'a> = Box<dyn Iterator<Item = &'a dyn matchers::Matcher> + 'a>;

fn parse_matcher(matcher_rule: pest::iterators::Pair<Rule>) -> Iter<'_> {
    Box::new(matcher_rule.into_inner().flat_map(|r| {
        match r.as_rule() {
            Rule::wildcardedDotChild => Box::new(iter::once(&matchers::WildcardedChild {} as &dyn matchers::Matcher)) as Iter<'_>,

            Rule::namedDotChild => Box::new(parse_dot_child_matcher(r)) as Iter<'_>,

            _ => Box::new(iter::empty()) as Iter<'_>
        }
    }))
}

fn parse_dot_child_matcher(
    matcher_rule: pest::iterators::Pair<Rule>,
) -> Iter<'_> {
    Box::new(matcher_rule.into_inner().flat_map(|r| {
        if let Rule::childName = r.as_rule() {
            Box::new(iter::once(&matchers::Child::new(r.as_str().to_owned()) as &dyn matchers::Matcher)) as Iter<'_>
        } else {
            Box::new(iter::empty()) as Iter<'_>
        }
    }))
}

fn parse_union(matcher_rule: pest::iterators::Pair<Rule>) -> Iter<'_> {
    Box::new(matcher_rule.into_inner().flat_map(|r| {
        if let Rule::unionChild = r.as_rule() {
            Box::new(parse_union_child(r)) as Iter<'_>
        } else {
            Box::new(iter::empty()) as Iter<'_>
        }
    }))
}

fn parse_union_child(matcher_rule: pest::iterators::Pair<Rule>) -> Iter<'_> {
    Box::new(matcher_rule.into_inner().flat_map(|r| {
        match r.as_rule() {
            Rule::doubleInner => {
                Box::new(iter::once(&matchers::Child::new(unescape(r.as_str())) as &dyn matchers::Matcher)) as Iter<'_>
            }

            Rule::singleInner => {
                Box::new(iter::once(&matchers::Child::new(unescape(r.as_str())) as &dyn matchers::Matcher)) as Iter<'_>
            }

            _ => Box::new(iter::empty()) as Iter<'_>
        }
    }))
}

const ESCAPED: &str = "\"'\\/bfnrt";
const UNESCAPED: &str = "\"'\\/\u{0008}\u{000C}\u{000A}\u{000D}\u{0009}";

fn unescape(contents: &str) -> String {
    let mut output = String::new();
    let xs: Vec<char> = contents.chars().collect();
    let mut i = 0;
    while i < xs.len() {
        if xs[i] == '\\' {
            i += 1;
            if xs[i] == 'u' {
                i += 1;

                // convert xs[i..i+4] to Unicode character and add it to the output
                let x = xs[i..i + 4].iter().collect::<String>();
                let n = u32::from_str_radix(&x, 16);
                let u = std::char::from_u32(n.unwrap());
                output.push(u.unwrap());

                i += 4;
            } else {
                for (j, c) in ESCAPED.chars().enumerate() {
                    if xs[i] == c {
                        output.push(UNESCAPED.chars().nth(j).unwrap())
                    }
                }
                i += 1;
            }
        } else {
            output.push(xs[i]);
            i += 1;
        }
    }
    output
}
