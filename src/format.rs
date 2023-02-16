// Format string
// %var%otherdata.%var%
// Pattern string
// {var:?}otherdata.{var:?}
// ? = any, ?w = /w, ?d = /d, ... (only 1 char)

use crate::{error::FOError, parser::format::any};
use std::{iter::Peekable, str::Chars};

use regex::Regex;

#[derive(Debug)]
pub struct FormatString {
    parts: Vec<StringPart>,
    pub vars: Vec<String>,
}

#[derive(Debug)]
enum StringPart {
    Literal(String),
    Variable(String),
}

impl FormatString {
    pub fn parse(format: &str) -> FormatString {
        let format = format.to_string();
        let mut vars = Vec::new();
        let parts: Vec<StringPart> = format
            .split("%")
            .enumerate()
            .filter_map(|(i, str)| {
                if str.eq("") {
                    return None;
                }
                if i % 2 == 0 {
                    Some(StringPart::Literal(str.to_string()))
                } else {
                    vars.push(str.to_string());
                    Some(StringPart::Variable(str.to_string()))
                }
            })
            .collect();
        FormatString { parts, vars }
    }

    pub fn generate_string(&self, vars: &Vec<String>) -> String {
        let mut result = String::new();
        let mut var_index = 0;
        for part in &self.parts {
            match part {
                StringPart::Literal(literal) => result.push_str(literal),
                StringPart::Variable(var) => {
                    result.push_str(vars.get(var_index).unwrap());
                    var_index += 1;
                }
            }
        }
        result
    }
}

#[derive(Debug)]
enum ParseResult {
    Literal(String),
    Variable(String),
    Capture(String, Option<String>),
    // usize = number of captures
    Any(String, usize),
}

impl ToString for ParseResult {
    fn to_string(&self) -> String {
        match self {
            ParseResult::Literal(literal) => literal.to_string(),
            ParseResult::Variable(var) => var.to_string(),
            ParseResult::Capture(result, _) => {
                format!("({})", result)
            }
            ParseResult::Any(result, _) => result.to_owned(),
        }
    }
}

#[derive(Debug)]
pub struct PatternString {
    regex: Regex,
    vars: Vec<Option<String>>,
}

impl PatternString {
    pub fn parse(pattern: &str, vars: Vec<Option<String>>) -> Result<PatternString, FOError> {
        // let mut vars = Vec::new();
        let mut i = 0;
        let (_, (varlist, regex_pattern)) =
            any(pattern).map_err(|e| FOError::PatternError(e.to_string()))?;
        let regex = Regex::new(&regex_pattern).unwrap();
        let varlist = varlist
            .into_iter()
            .map(|d| {
                // d = &None;
                if d.is_none() {
                    i += 1; // fuck this will deal with it later
                    match vars.get(i - 1) {
                        Some(var) if var.is_some() => Some(var.as_ref().unwrap().to_owned()),
                        _ => None,
                    }
                } else {
                    d
                }
            })
            .collect();
        Ok(PatternString {
            regex,
            vars: varlist,
        })
    }

    /// Result: (varname, capture)
    pub fn get_data(&self, input: &str) -> Option<Vec<(String, String)>> {
        let mut result = Vec::new();
        let captures = self.regex.captures(input)?;
        for (i, var) in self.vars.iter().enumerate() {
            if var.is_some() {
                result.push((
                    var.as_ref().unwrap().to_string(),
                    captures.get(i + 1).unwrap().as_str().to_string(),
                ));
            }
        }
        Some(result)
    }
}

#[test]
fn test_formatstring() {
    let format = FormatString::parse("%var%otherdata.%var%");
    println!("{:?}", format);
    println!(
        "{}",
        format.generate_string(&vec!["test".to_string(), "test2".to_string()])
    );
    let format2 = FormatString::parse("a%var%%var2%we");
    println!("{:?}", format2);
    let str = "hello world";
    let sliced = &str[0..2];
    println!("{}", sliced);
}

#[test]
fn test_parser() {
    let input = (
        "{?}.{mp3|mp4}",
        vec![Some("var".to_string()), Some("ext".to_string())],
    );
    let pattern = PatternString::parse(input.0, input.1).unwrap();
    dbg!(pattern.get_data("test.mp5"));
    // println!("{:?}", pattern);
}
