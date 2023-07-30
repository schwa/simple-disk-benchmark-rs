use crate::colored_markup::*;
use anyhow::Result;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, char, multispace0},
    combinator::{map, opt, value},
    error::ParseError,
    multi::{many0, many1, separated_list0},
    sequence::{delimited, tuple},
    IResult, Parser,
};

/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and
/// trailing whitespace, returning the output of `inner`.
fn ws<'a, F, O, E: ParseError<&'a str>>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: Parser<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

pub(crate) fn parse(s: &str) -> Result<Vec<(&str, Style)>> {
    // TODO: Handle unconsumed input.
    rules(s)
        .or(Err(anyhow::anyhow!("Failed to parse stylesheet")))
        .map(|(_, rules)| rules)
}

#[test]
fn test_parse() {
    assert_eq!(
        parse(" r { foreground: red; }").unwrap(),
        vec![("r", Style::new(None, Some(Color::Red), None))]
    );
}

fn rules(s: &str) -> IResult<&str, Vec<(&str, Style)>> {
    many0(rule)(s)
}

#[test]
fn test_rules() {
    assert_eq!(
        rules("alert{foreground:red}").unwrap().1,
        vec![("alert", Style::new(None, Some(Color::Red), None))]
    );
}

fn rule(s: &str) -> IResult<&str, (&str, Style)> {
    map(
        tuple((ws(selector), wrapped_declarations)),
        |(selector, style)| (selector, style),
    )(s)
}

#[test]
fn test_rule() {
    assert_eq!(
        rule("alert{foreground:red}").unwrap().1,
        ("alert", Style::new(None, Some(Color::Red), None))
    );
    assert_eq!(
        rule("alert { foreground: red; background: blue }")
            .unwrap()
            .1,
        (
            "alert",
            Style::new(None, Some(Color::Red), Some(Color::Blue))
        )
    );
    assert_eq!(
        rule("alert{foreground:red;}").unwrap().1,
        ("alert", Style::new(None, Some(Color::Red), None))
    );
}

fn selector(s: &str) -> IResult<&str, &str> {
    alpha1(s)
}

fn wrapped_declarations(s: &str) -> IResult<&str, Style> {
    map(
        tuple((ws(char('{')), declarations, opt(char(';')), ws(char('}')))),
        |(_, style, _, _)| style,
    )(s)
}

#[test]
fn test_wrapped_declarations() {
    assert_eq!(
        wrapped_declarations("{ foreground: red }").unwrap().1,
        Style::new(None, Some(Color::Red), None)
    );
    assert_eq!(
        wrapped_declarations("{ foreground: red; styles: bold }")
            .unwrap()
            .1,
        Style::new(Some(vec![Styles::Bold]), Some(Color::Red), None)
    );
    assert_eq!(
        wrapped_declarations("{ foreground: red; styles: bold }")
            .unwrap()
            .1,
        Style::new(Some(vec![Styles::Bold]), Some(Color::Red), None)
    );
}

fn declarations(s: &str) -> IResult<&str, Style> {
    map(separated_list0(char(';'), declaration), |decls| {
        let mut result = Style::empty();
        for style in decls {
            result = result.merge(style);
        }
        result
    })(s)
}

fn declaration(s: &str) -> IResult<&str, Style> {
    alt((color_style_declaration, styles_style_declaration))(s)
}

#[test]
fn test_declaration() {
    assert_eq!(
        color_style_declaration("foreground: red").unwrap().1,
        Style::new(None, Some(Color::Red), None)
    );
    assert_eq!(
        styles_style_declaration("styles : bold").unwrap().1,
        Style::new(Some(vec![Styles::Bold]), None, None)
    );
    assert_eq!(
        styles_style_declaration("styles : bold dimmed").unwrap().1,
        Style::new(Some(vec![Styles::Bold, Styles::Dimmed]), None, None)
    );
}

fn color_style_declaration(s: &str) -> IResult<&str, Style> {
    let foreground_or_background = alt((tag("foreground"), tag("background")));
    map(
        tuple((ws(foreground_or_background), char(':'), ws(color))),
        |(attribute, _, color)| match attribute {
            "foreground" => Style::new(None, Some(color), None),
            "background" => Style::new(None, None, Some(color)),
            _ => panic!(),
        },
    )(s)
}

fn styles_style_declaration(s: &str) -> IResult<&str, Style> {
    map(
        tuple((ws(tag("styles")), char(':'), many1(ws(styles)))),
        |(_, _, styles)| Style::new(Some(styles), None, None),
    )(s)
}

fn styles(s: &str) -> IResult<&str, Styles> {
    alt((
        value(Styles::Bold, tag("bold")),
        value(Styles::Dimmed, tag("dimmed")),
        value(Styles::Underline, tag("underline")),
        value(Styles::Reversed, tag("reversed")),
        value(Styles::Italic, tag("italic")),
        value(Styles::Blink, tag("blink")),
        value(Styles::Hidden, tag("hidden")),
        value(Styles::Strikethrough, tag("strikethrough")),
    ))(s)
}
#[test]
fn test_styles() {
    assert_eq!(styles("bold").unwrap().1, Styles::Bold);
}

fn color(s: &str) -> IResult<&str, Color> {
    alt((
        value(Color::Black, tag("black")),
        value(Color::Red, tag("red")),
        value(Color::Green, tag("green")),
        value(Color::Yellow, tag("yellow")),
        value(Color::Blue, tag("blue")),
        value(Color::Magenta, tag("magenta")),
        value(Color::Cyan, tag("cyan")),
        value(Color::White, tag("white")),
        value(Color::BrightBlack, tag("bright-black")),
        value(Color::BrightRed, tag("bright-red")),
        value(Color::BrightGreen, tag("bright-green")),
        value(Color::BrightYellow, tag("bright-yellow")),
        value(Color::BrightBlue, tag("bright-blue")),
        value(Color::BrightMagenta, tag("bright-magenta")),
        value(Color::BrightCyan, tag("bright-cyan")),
        value(Color::BrightWhite, tag("bright-white")),
    ))(s)
}

#[test]
fn test_color() {
    assert_eq!(color("red").unwrap().1, Color::Red);
}
