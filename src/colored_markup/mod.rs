#![allow(dead_code)]

use anyhow::{anyhow, Ok, Result};
use colored::ColoredString;
use colored::Colorize;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

type Color = colored::Color;

mod stylesheet;

use stylesheet::parse;

#[derive(Debug, PartialEq)]
enum Part<'a> {
    OpenTag(&'a str),
    CloseTag(&'a str),
    Text(&'a str),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Styles {
    Bold,
    Dimmed,
    Underline,
    Reversed,
    Italic,
    Blink,
    Hidden,
    Strikethrough,
}

impl Styles {
    fn apply(&self, s: colored::ColoredString) -> colored::ColoredString {
        match self {
            Styles::Bold => s.bold(),
            Styles::Dimmed => s.dimmed(),
            Styles::Underline => s.underline(),
            Styles::Reversed => s.reversed(),
            Styles::Italic => s.italic(),
            Styles::Blink => s.blink(),
            Styles::Hidden => s.hidden(),
            Styles::Strikethrough => s.strikethrough(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Style {
    styles: Vec<Styles>, // TODO: Hashset
    foreground: Option<colored::Color>,
    background: Option<colored::Color>,
}

impl Style {
    fn new(
        styles: Option<Vec<Styles>>,
        foreground: Option<colored::Color>,
        background: Option<colored::Color>,
    ) -> Style {
        Style {
            styles: styles.unwrap_or_default(),
            foreground,
            background,
        }
    }

    fn empty() -> Style {
        Style {
            styles: Vec::new(),
            foreground: None,
            background: None,
        }
    }

    fn merge(&self, other: Style) -> Style {
        let mut styles = self.styles.clone();
        styles.extend(other.styles);
        Style {
            styles,
            foreground: other.foreground.or(self.foreground),
            background: other.background.or(self.background),
        }
    }

    fn resolve(stack: &Vec<Style>) -> Style {
        let mut styles: Vec<Styles> = Vec::new();
        let mut foreground: Option<colored::Color> = None;
        let mut background: Option<colored::Color> = None;

        for style in stack {
            styles.extend(style.styles.iter());
            if style.foreground.is_some() {
                foreground = style.foreground;
            }
            if style.background.is_some() {
                background = style.background;
            }
        }

        // styles = styles
        //     .iter()
        //     .unique_by(|s| s.to_string())
        //     .cloned()
        //     .collect();

        Style {
            styles,
            foreground,
            background,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct StyleSheet<'a> {
    styles: HashMap<&'a str, Style>,
}

impl<'a> Default for StyleSheet<'a> {
    fn default() -> StyleSheet<'a> {
        let styles = vec![
            ("bold", Style::new(Some(vec![Styles::Bold]), None, None)),
            ("em", Style::new(Some(vec![Styles::Italic]), None, None)),
            (
                "strikethrough",
                Style::new(Some(vec![Styles::Strikethrough]), None, None),
            ),
        ];
        StyleSheet::new(&styles)
    }
}

impl<'a> StyleSheet<'a> {
    fn new(styles: &[(&'a str, Style)]) -> StyleSheet<'a> {
        let styles = HashMap::from_iter(styles.iter().cloned());
        StyleSheet { styles }
    }
}

impl<'a> StyleSheet<'a> {
    pub fn parse(s: &'a str) -> Result<StyleSheet<'a>> {
        let rules = parse(s)?;
        Ok(StyleSheet::new(&rules))
    }
}

#[test]
fn test_stylesheet() {
    let styles = vec![("alert", Style::new(None, Some(colored::Color::Red), None))];
    let expectation = StyleSheet::new(&styles);
    assert_eq!(
        StyleSheet::parse("alert{foreground:red}").unwrap(),
        expectation
    );
}

impl StyleSheet<'_> {
    fn parse_template(t: &str) -> Vec<Part> {
        lazy_static! {
            static ref REGEX: Regex = Regex::new(
                r"(?x)
                (?P<tag><
                    (?:(?P<open>[a-z]+)|/(?P<close>[a-z]+))
                >)"
            )
            .unwrap();
        }
        let mut parts: Vec<Part> = Vec::new();
        let mut current_index: usize = 0;
        while let Some(captures) = REGEX.captures_at(t, current_index) {
            if let Some(tag) = captures.name("tag") {
                let text = &t[current_index..tag.start()];
                if !text.is_empty() {
                    parts.push(Part::Text(text));
                }
                current_index = tag.end();
                if let Some(open) = captures.name("open") {
                    parts.push(Part::OpenTag(open.as_str()));
                } else if let Some(close) = captures.name("close") {
                    parts.push(Part::CloseTag(close.as_str()));
                }
            }
        }
        let text = &t[current_index..];
        if !text.is_empty() {
            parts.push(Part::Text(text));
        }
        parts
    }

    pub fn render(&self, t: &str) -> Result<String> {
        let parts = StyleSheet::parse_template(t);

        let mut style_stack: Vec<Style> = Vec::new();

        let mut colored_strings: Vec<colored::ColoredString> = Vec::new();

        for part in parts {
            match part {
                Part::Text(text) => {
                    let style = Style::resolve(&style_stack);
                    let mut text = ColoredString::from(text);

                    for style in style.styles {
                        text = style.apply(text);
                    }

                    if let Some(color) = style.foreground {
                        text = text.color(color);
                    }
                    if let Some(color) = style.background {
                        text = text.on_color(color);
                    }
                    colored_strings.push(text);
                }
                Part::OpenTag(tag) => {
                    if let Some(style) = self.styles.get(tag) {
                        style_stack.push(style.clone());
                    } else {
                        style_stack.push(Style::default());
                    }
                }
                Part::CloseTag(_) => {
                    style_stack
                        .pop()
                        .ok_or_else(|| anyhow!("Invalid template"))?; // TODO: error
                }
            }
        }

        let mut result = String::new();
        for colored_string in colored_strings {
            let f = format!("{}", colored_string);
            result.push_str(&f);
        }
        Ok(result)
    }
}

#[macro_export]
macro_rules! cmarkup {
    ($template:tt, $($arg:tt)*) => {{
        let s = format!($($arg)*);
        $template.render(&s).unwrap()
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_template() {
        let parts = StyleSheet::parse_template("Hello <bold>World</bold><em></em>!");
        let expectation = vec![
            Part::Text("Hello "),
            Part::OpenTag("bold"),
            Part::Text("World"),
            Part::CloseTag("bold"),
            Part::OpenTag("em"),
            Part::CloseTag("em"),
            Part::Text("!"),
        ];
        assert_eq!(parts, expectation);
    }

    #[test]
    fn test_no_styles_template() {
        let template = StyleSheet {
            styles: HashMap::new(),
        };
        let result = template.render("Hello <bold>World</bold><em></em>!");
        assert_eq!(result.unwrap(), "Hello World!");
    }

    // TODO: Disable because this fails in github. Need to force color output.
    // #[test]
    // fn test_template() {
    //     let template = Template::default();
    //     let result = template.render("<em>EM <bold>BOLD</bold>EM</em>").unwrap();
    //     //println!("{}", result);
    //     assert_eq!(
    //         result,
    //         "\u{1b}[3mEM \u{1b}[0m\u{1b}[1;3mBOLD\u{1b}[0m\u{1b}[3mEM\u{1b}[0m"
    //     );
    // }
}
