// Format string
// %var%otherdata.%var%
// Pattern string
// {var:?}otherdata.{var:?}
// ? = any, ?w = /w, ?d = /d, ... (only 1 char)

use std::{iter::Peekable, str::Chars};

use regex::Regex;

#[derive(Debug)]
struct FormatString {
    parts: Vec<StringPart>,
    vars: Vec<String>,
}

#[derive(Debug)]
enum StringPart {
    Literal(String),
    Variable(String),
}

impl FormatString {
    fn parse(format: &str) -> FormatString {
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

    fn generate_string(&self, vars: &Vec<String>) -> String {
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
struct ParserHelper<'a> {
    iter: Chars<'a>,
    // iter: Peekable<Chars<'a>>,
    children: Vec<Option<String>>,
    index: i32,
}

impl<'a> ParserHelper<'a> {
    fn new(input: &str) -> ParserHelper {
        // let types = Box::new(input.chars().peekable());
        // let types = input.chars();
        ParserHelper {
            iter: input.chars(),
            children: Vec::new(),
            index: -1,
        }
    }

    fn peek(&mut self) -> Option<char> {
        dbg!(
            "peek",
            &((self.index + 1) as usize),
            &self.iter.nth((self.index + 1) as usize).clone()
        );
        self.iter.nth((self.index + 1) as usize).clone()
        // self.iter.peek().cloned()
    }

    fn next(&mut self) -> Option<char> {
        self.index += 1;
        dbg!(
            "next",
            &self.index,
            &self.iter.nth(self.index as usize).clone()
        );
        self.iter.nth(self.index as usize).clone()
    }

    fn add_var(&mut self, var: Option<String>, count: usize) {
        dbg!(self.children.len() - count);
        self.children.insert(self.children.len() - count, var)
    }
}

fn parser_var(input: &mut ParserHelper) -> Option<ParseResult> {
    let mut result = String::new();
    // remove space
    while matches!(input.peek(), Some(' ')) {
        input.next();
    }
    // get non weird chars
    let ignore_list = ['{', '}', ':', ' ', '?'];
    let mut ticker = false;
    while let Some(a) = input.peek() {
        dbg!(a);
        if a.eq(&'\\') {
            ticker = true;
            input.next();
            result.push_str(&input.next().unwrap().to_string());
            continue;
        } else if ignore_list.contains(&a) {
            break;
        } else {
            ticker = true;
            result.push_str(&input.next().unwrap().to_string());
        }
    }
    // remove space
    while matches!(input.peek(), Some(' ')) {
        input.next();
    }
    if ticker {
        Some(ParseResult::Variable(result))
    } else {
        None
    }
    // ParseResult::Variable(result)
}

fn parser_any(input: &mut ParserHelper) -> Option<ParseResult> {
    let func_order = [parser_capture, parser_metacharacter, parser_literal];
    let mut result = String::new();
    let mut captures = 0;
    while let Some(a) = input.peek() {
        if a.eq(&'}') {
            break;
        }
        for func in &func_order {
            if let Some(res) = func(input) {
                match res {
                    ParseResult::Literal(literal) => result.push_str(&literal),
                    ParseResult::Variable(var) => result.push_str(&var),
                    ParseResult::Capture(capture, var) => {
                        captures += 1;
                        result.push_str(&capture);
                        if let Some(var) = var {
                            input.add_var(Some(var), 0);
                        } else {
                            input.add_var(None, 0);
                        }
                    }
                    _ => (),
                }
                break;
            }
        }
    }
    Some(ParseResult::Any(result, captures))
}

fn parser_literal(input: &mut ParserHelper) -> Option<ParseResult> {
    let mut result = String::new();
    let ignore_list = ['{', '}', ':', ' ', '?'];
    while let Some(a) = input.peek() {
        if a.eq(&'\\') {
            input.next();
            result.push_str(&input.next().unwrap().to_string());
            continue;
        } else if ignore_list.contains(&a) {
            break;
        } else {
            result.push_str(&input.next().unwrap().to_string());
        }
    }
    Some(ParseResult::Literal(result))
}

fn parser_metacharacter(input: &mut ParserHelper) -> Option<ParseResult> {
    // remove ?
    if !matches!(input.next(), Some('?')) {
        return None;
    }
    // match
    let metacharlist = ['.', 'w', 'W', 'd', 'D', 'b', 'B', 's', 'S'];
    let char = input.peek()?;
    if metacharlist.contains(&char) {
        input.next();
        if !char.eq(&'.') {
            Some(ParseResult::Literal(format!("\\{}+?", char)))
        } else {
            Some(ParseResult::Literal(".+?".to_owned()))
        }
    } else {
        Some(ParseResult::Literal(".+?".to_owned()))
    }
}

fn parser_capture(input: &mut ParserHelper) -> Option<ParseResult> {
    let var;
    let mut result = String::new();
    // remove {
    if !matches!(input.next(), Some('{')) {
        return None;
    }
    // try to get var name
    // TODO: make it actually try not just do it
    if let ParseResult::Variable(var_parsed) = parser_var(input)? {
        if matches!(input.peek(), Some(':')) {
            input.next();
            var = Some(var_parsed.to_string());
        } else {
            var = None;
        }
        // result.push_str(&parsed);
    } else {
        var = None;
    }
    // get any
    if let ParseResult::Any(parsed, count) = parser_any(input)? {
        // remove }
        if matches!(input.next(), Some('}')) {
            result.push_str(&parsed.to_string());
            input.add_var(var.clone(), count);
        } else {
            return None;
        }
    }

    Some(ParseResult::Capture(result, var))
}

struct PatternString {
    regex: Regex,
    vars: Vec<String>,
}

impl PatternString {
    fn parse(pattern: &str) -> PatternString {
        let mut vars = Vec::new();
        let regex_pattern: String = String::new();
        // let unknown = pattern.chars().for_each(|c|{
        //     if c == '{'
        // });
        let regex = Regex::new(&regex_pattern).unwrap();
        PatternString { regex, vars }
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
    let text = "{abc}";
    let mut parser = ParserHelper::new(text);
    dbg!(parser_capture(&mut parser), parser);
}
