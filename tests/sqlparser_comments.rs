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

#![warn(clippy::all)]
//! Test comment extraction from SQL source code.

#[cfg(test)]
use pretty_assertions::assert_eq;

use sqlparser::{
    ast::{
        comments::{Comment, CommentWithSpan, Comments},
        Spanned, Statement,
    },
    dialect::GenericDialect,
    parser::Parser,
    tokenizer::{Location, Span},
};

#[test]
fn parse_sql_with_comments() {
    let sql = r#"
-- second line comment
select * from /* inline comment after `from` */ dual;

/*select
some
more*/

  -- end-of-script-with-no-newline"#;

    let comments = match Parser::parse_sql_with_comments(&GenericDialect, sql) {
        Ok((_, comments)) => comments,
        Err(e) => panic!("Invalid sql script: {e}"),
    };

    assert_eq!(
        Vec::from(comments),
        vec![
            CommentWithSpan {
                comment: Comment::SingleLine {
                    content: " second line comment".into(),
                    prefix: "--".into()
                },
                span: Span::new((2, 1).into(), (2, 23).into()),
                prev_token_end: None,
                next_token_start: Some(Location::new(3, 1)),
            },
            CommentWithSpan {
                comment: Comment::MultiLine(" inline comment after `from` ".into()),
                span: Span::new((3, 15).into(), (3, 48).into()),
                prev_token_end: Some(Location::new(3, 14)),
                next_token_start: Some(Location::new(3, 49)),
            },
            CommentWithSpan {
                comment: Comment::MultiLine("select\nsome\nmore".into()),
                span: Span::new((5, 1).into(), (7, 7).into()),
                prev_token_end: Some(Location::new(3, 54)),
                next_token_start: None,
            },
            CommentWithSpan {
                comment: Comment::SingleLine {
                    content: " end-of-script-with-no-newline".into(),
                    prefix: "--".into()
                },
                span: Span::new((9, 3).into(), (9, 35).into()),
                prev_token_end: Some(Location::new(3, 54)),
                next_token_start: None,
            }
        ]
    );
}

fn starts_line(comment: &CommentWithSpan) -> bool {
    comment
        .prev_token_end
        .is_none_or(|end| end.line < comment.span.start.line)
}

fn second_column_start(sql: &str) -> (Location, Comments) {
    let (statements, comments) = Parser::parse_sql_with_comments(&GenericDialect, sql).unwrap();
    let Statement::CreateTable(table) = &statements[0] else {
        panic!("expected CREATE TABLE");
    };
    (table.columns[1].span().start, comments)
}

#[test]
fn preceding_trailing_comment_does_not_start_line() {
    let (name_start, comments) =
        second_column_start("CREATE TABLE t (\n  id INT, -- about id\n  name TEXT\n)");
    let found: Vec<_> = comments.preceding(name_start).collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].as_str(), " about id");
    assert!(!starts_line(found[0]));
}

#[test]
fn preceding_standalone_comment_starts_line() {
    let (name_start, comments) =
        second_column_start("CREATE TABLE t (\n  id INT,\n  -- about name\n  name TEXT\n)");
    let found: Vec<_> = comments.preceding(name_start).collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].as_str(), " about name");
    assert!(starts_line(found[0]));
}

#[test]
fn preceding_stops_at_intervening_tokens() {
    let sql = "-- about a\nCREATE TABLE a (x INT); CREATE TABLE b (y INT)";
    let (_, comments) = Parser::parse_sql_with_comments(&GenericDialect, sql).unwrap();
    let a_create_start = Location::new(2, 1);
    let b_create_start = Location::new(2, 25);
    let found: Vec<_> = comments.preceding(a_create_start).collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].as_str(), " about a");
    assert!(starts_line(found[0]));
    assert_eq!(comments.preceding(b_create_start).count(), 0);
}

#[test]
fn preceding_returns_every_comment_between_two_tokens_in_order() {
    let sql = "SELECT 1; /* a */\n\n-- b\n/* c */ SELECT 2";
    let (_, comments) = Parser::parse_sql_with_comments(&GenericDialect, sql).unwrap();
    let found: Vec<_> = comments.preceding(Location::new(4, 9)).collect();
    assert_eq!(
        found.iter().map(|c| c.as_str()).collect::<Vec<_>>(),
        [" a ", " b", " c "]
    );
    assert_eq!(
        found.iter().map(|c| starts_line(c)).collect::<Vec<_>>(),
        [false, true, true]
    );
    assert!(found
        .iter()
        .all(|c| c.prev_token_end == Some(Location::new(1, 10))
            && c.next_token_start == Some(Location::new(4, 9))));
    // not a token start
    assert_eq!(comments.preceding(Location::new(4, 10)).count(), 0);
}
