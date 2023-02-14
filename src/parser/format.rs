use nom::{
    self,
    branch::alt,
    bytes::complete::{escaped, tag, take_till1, take_until, take_until1, take_while1},
    character::complete::{none_of, one_of, space0, space1},
    combinator::{flat_map, map, opt},
    multi::{many1, separated_list1},
    sequence::{delimited, pair, preceded, separated_pair, terminated},
    IResult,
};

fn text(input: &str) -> IResult<&str, &str> {
    // let i = take_till1(|c| ignore_char.contains(&c))(input)?;
    let data = escaped(none_of(r#"\{}:?"#), '\\', one_of(r#"{}:\?"#))(input);
    data.and_then(|s| {
        if s.1.is_empty() {
            Err(nom::Err::Error(nom::error::Error::new(
                s.0,
                nom::error::ErrorKind::Tag,
            )))
        } else {
            Ok(s)
        }
    })
    // data
    // data
}

fn varname(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)
}

fn wildcard(input: &str) -> IResult<&str, String> {
    map(preceded(tag("?"), opt(one_of("wWdD"))), |d| match d {
        Some(d) => format!("\\{}+?", d),
        None => ".+?".to_owned(),
    })(input)
}

fn capture(input: &str) -> IResult<&str, (Vec<Option<&str>>, String)> {
    delimited(
        tag("{"),
        map(
            pair(
                opt(terminated(
                    opt(delimited(space0, varname, space0)),
                    tag(":"),
                )),
                any,
            ),
            |(var, mut res)| {
                res.0.insert(0, var.unwrap_or(None));
                (res.0, format!("({})", res.1))
            },
        ),
        tag("}"),
    )(input)
}

fn capture_or(input: &str) -> IResult<&str, (Vec<Option<&str>>, String)> {
    delimited(
        tag("{"),
        map(separated_list1(tag("|"), any), |d| {
            let mut result = vec![];
            let mut varlist = vec![None];
            for (var, str) in d {
                result.push(str);
                varlist.extend(var);
            }
            (varlist, format!("({})", result.join("|")))
        }),
        tag("}"),
    )(input)
}

fn any(input: &str) -> IResult<&str, (Vec<Option<&str>>, String)> {
    map(
        many1(alt((
            map(wildcard, |d| (vec![], d)),
            capture_or,
            capture,
            map(text, |d| (vec![], d.to_owned())),
        ))),
        |d| {
            let mut result = String::new();
            let mut varlist = vec![];
            for (var, str) in d {
                result.push_str(&str);
                varlist.extend(var);
            }
            (varlist, result)
        },
    )(input)
}

#[test]
fn test_parser() {
    let input = r#"lalala wew s \\\{were{"#;
    let result = text(input);
    dbg!(result);
    let wildcardin = r#"?Aweqweqw"#;
    let result = wildcard(wildcardin);
    dbg!(result);
    let capturein = r#"{var:?test?d}"#;
    let result = capture(capturein);
    dbg!(result);
    let any_input = r#"{ anime   :?}.{mp4|mp3}"#;
    let result = any(any_input);
    dbg!(result);
}
