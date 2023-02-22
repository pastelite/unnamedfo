// What I need to do
// 1. Read file and their config
// 2. Loop through file in each folder and make sure it fit schema
// 3. If it don't, "move" it

// How to move
// 1. read import config line by line until match
// if not match, put it in _uncategorized
// 2. if it fit some config line, loop through schema
// 	is current data field fit schema?
// 	if true
// 		if true, remove datafield and loop through scheme again
// 		if false, return error
// 	if false/error go to next schema

use std::{
    collections::HashMap,
    fmt::{Debug, Display, Formatter},
    path::{Path, PathBuf},
};

// use async_std::path::PathBuf;

use crate::{
    config_reader::Config,
    error::FOError,
    format::PatternString,
    schema::{self, Schema, SchemaList},
};

pub struct Mover {
    path: PathBuf,
}

impl Mover {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_owned(),
        }
    }

    pub fn get_path(
        &self,
        config: &Config,
        schemalist: &SchemaList,
        // schemanames: &Vec<&str>,
    ) -> Result<String, FOError> {
        // read import config
        for (pattern, var) in &config.import.list {
            // deal with commaseperated
            let mut vars = vec![];
            for var in &var.0 {
                if var.eq("_") || var.eq("") {
                    vars.push(None)
                } else {
                    vars.push(Some(var.to_owned()))
                }
            }

            // matching the pattern
            let pat = PatternString::parse(pattern, vars)?;
            let matches = pat.get_data(
                self.path
                    .file_name()
                    .ok_or(FOError::PatternError("Error converting OsStr".to_owned()))?
                    .to_str()
                    .unwrap(),
            );
            match matches {
                Some(matches) => {
                    let meta = config.get_meta(schemalist);
                    let schema_names = meta
                        .other
                        .children
                        .0
                        .iter()
                        .map(|f| f.as_str())
                        .collect::<Vec<&str>>();

                    let movetree = schema_finder(&schemalist, &schema_names, &matches);

                    dbg!(&movetree);
                    return Ok(movetree.unwrap().to_path(&schemalist));
                    // println!("move to {}", movetree.unwrap().to_path(&schemalist));
                }
                None => {
                    continue;
                }
            }
        }
        Err(FOError::PatternError("No pattern match".to_owned()))
    }
}

#[test]
fn test_mover() {
    let config = r#"
        _meta:
            children: anime
        _schema:
            anime:
               filename: '%name%'
               children: file
               fields: name
            file:
                filename: '%filename%.%ext%'
                fields: filename, ext
            root:
                children: anime
        _import:
            - "{?}-{?}.{mp4|mp3}": name, filename, ext
    "#;

    let parsed_config = serde_yaml::from_str::<Config>(&config).unwrap();
    let schemalist = SchemaList::from(&parsed_config.schema);
    dbg!(&schemalist);

    // let schemanames = parsed_config.me

    // let schemalist = SchemaList::parse_config(&mut self, name, config)
    let moves = Mover::new("./testdir/test-ad.mp4");
    moves.get_path(&parsed_config, &schemalist).unwrap();
}

fn schema_finder(
    schemalist: &SchemaList,
    // schemaname: &str,
    schemalist_name: &Vec<&str>,
    data: &Vec<(String, String)>,
) -> Option<MoveTree> {
    let schemas_from_names = schemalist_name
        .iter()
        .filter_map(|name| schemalist.get(name))
        .collect::<Vec<&Schema>>();

    for schema in schemas_from_names {
        let related_data = prune_unrelated_data(data, &schema.name);
        if schema.is_fit(&related_data) {
            let (data_pruned, data_rest) = prune_data(&data, &schema);

            let schema_names = schemalist
                .get_children(&schema.name)
                .unwrap_or_default()
                .iter()
                .map(|d| {
                    let tes = d.name.as_str();
                    tes
                })
                .collect::<Vec<&str>>();

            let a = schema_finder(schemalist, &schema_names, &data_rest);
            match a {
                None => continue,
                Some(a) => {
                    return Some(MoveTree {
                        name: schema.name.clone(),
                        fields: remove_dot(&data_pruned),
                        children: Some(Box::new(a)),
                    });
                }
            }
        }
    }
    Some(MoveTree {
        name: "_Uncategorized".to_owned(),
        fields: remove_dot(&data),
        children: None,
    })
}

fn match_text(full: &str, short: &str) -> bool {
    let full = full.to_lowercase();
    let short = short.to_lowercase();
    let mut full = full.chars();
    let mut short = short.chars();

    loop {
        match (&full.next(), &short.next()) {
            (Some(f), Some(s)) if !f.eq(s) => return false,
            (None, _) | (_, None) => break,
            _ => continue,
        }
    }
    true
}

fn remove_dot(data: &Vec<(String, String)>) -> Vec<(String, String)> {
    let mut res = vec![];
    for (field, data) in data {
        if field.contains(".") {
            res.push((field.split(".").nth(1).unwrap().to_owned(), data.to_owned()))
        } else {
            res.push((field.to_owned(), data.to_owned()))
        }
    }
    res
}

#[test]
fn test_text_match() {
    dbg!(match_text("anime", "an"));
    dbg!(match_text("Anime", "ani"));
    dbg!(match_text("anime", "book"));
    dbg!(match_text("an", "anime"));
}

fn prune_unrelated_data(data: &Vec<(String, String)>, schema_name: &str) -> Vec<(String, String)> {
    let mut res = vec![];
    for (field, data) in data {
        if field.contains(".") {
            let splited = field.split(".").nth(0).unwrap();
            if match_text(schema_name, splited) {
                res.push((field.split(".").nth(1).unwrap().to_owned(), data.to_owned()))
            }
        } else {
            res.push((field.to_owned(), data.to_owned()))
        }
    }
    res
}

fn prune_data(
    data: &Vec<(String, String)>,
    schema: &Schema,
) -> (Vec<(String, String)>, Vec<(String, String)>) {
    let mut data = data.clone();
    let schema_fields_key = schema.fields.keys();
    let schema_fields = schema_fields_key.map(|d| d.as_str()).collect::<Vec<&str>>();

    let (data_pruned, data_rest) = data.into_iter().partition(|(key, _)| {
        if key.contains(".") {
            let schema_name = key.split(".").nth(0).unwrap();
            let data_field = key.split(".").nth(1).unwrap();

            if match_text(schema_name, &schema.name) {
                return schema_fields.contains(&data_field);
            } else {
                return false;
            }
            // return schema_fields.contains(&k);
        } else {
            schema_fields.contains(&key.as_str())
        }
    });
    (data_pruned, data_rest)
}

#[test]
fn test_prune() {
    let data = vec![
        ("anim.name".to_owned(), "Naruto".to_owned()),
        ("epinum".to_owned(), "1".to_owned()),
        ("book.name".to_owned(), "Harry Potter".to_owned()),
    ];
    dbg!(prune_unrelated_data(&data, "anime"));

    let mut sl = SchemaList::new();
    sl.parse_format("Anime".to_owned(), "name epinum | |");
    dbg!(prune_data(&data, sl.list.get("Anime").unwrap()));
}

// macro_rules! S {
//     ($($var:expr),*) => {
//         ($(stringify!($var).to_owned() ),*)
//     };
// }

macro_rules! string {
    (($($var:expr),*)) => {
        ($(string!($var)),*)
    };
    ($var:expr) => {
        $var.to_owned()
    };
    ($($var:expr),+) => {
        ($(string!($var)),+)
    };
}

// fn test_s() {
//     let types = string!("a", "b");
//     let type2 = &types;
// }

// #[derive(Debug)]
struct MoveTree {
    name: String,
    fields: Vec<(String, String)>,
    children: Option<Box<MoveTree>>,
}

impl Debug for MoveTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // let str_formatter = f.deb
        // let tuple_formatter = f.debug_tuple(&self.name);
        // self.fields.iter().for_each(|(k, v)| {
        //     tuple_formatter.field(&format!("{}:{}", k, v));
        // });
        write!(f, "{}", self.name)?;
        write!(
            f,
            "[{}]",
            self.fields
                .iter()
                .map(|(k, v)| format!("{}:{}", k, v))
                .collect::<Vec<String>>()
                .join(",")
        )?;
        if let Some(children) = &self.children {
            write!(f, " -> {:?}", children)?;
        }
        Ok(())
    }
}

impl MoveTree {
    fn to_path(&self, sl: &SchemaList) -> String {
        // TODO: support lower char

        let schema = match sl.get(&self.name) {
            Some(s) => s,
            None => return String::new(),
        };

        let str = schema.generate_string(&self.fields);
        if self.children.is_some() {
            let child_path = self.children.as_ref().unwrap().to_path(sl);
            if child_path.is_empty() {
                str
            } else {
                format!("{}/{}", str, child_path)
            }
            // format!("{}/{}", str, self.children.as_ref().unwrap().to_path(sl))
        } else {
            str
        }
    }
}

#[test]
fn finder_test() {
    let mut schemalist = SchemaList::new();
    schemalist.parse_format(
        "anime".to_owned(),
        "name! epinum(num) | file | [Anime] %name% ",
    );
    // schemalist.parse_format("Video".to_owned(), "tags | |  ");
    schemalist.parse_format("Books".to_owned(), "name! artist | page | [Book] %name% ");
    schemalist.parse_format("page".to_owned(), "number(num) ext | | %num%.%ext% ");
    schemalist.parse_format("root".to_owned(), " | anime books | root ");
    schemalist.parse_format("file".to_owned(), "tags name ext | | %name%.%ext% ");

    let anime_data = vec![
        string!("anim.name", "Naruto"),
        string!("b.epinum", "1"),
        string!("tags", "Anime"),
        string!("file.name", "lol"),
        string!("ext", "mp4"),
    ];

    let book_data = vec![
        string!("book.name", "Math"),
        string!("lol.artist", "John"),
        string!("tags", "Book"),
        string!("number", "1"),
        string!("ext", "pdf"),
        // string!("ext", "Book"),
    ];
    // let book_data = string_vec!(("book.name", "Math"), ("artist", "John"), ("tags", "Book"));
    // let anime_data2 = &vec![
    //     ("book.name".to_owned(), "Naruto".to_owned()),
    //     ("epinum".to_owned(), "1".to_owned()),
    //     ("tags".to_owned(), "Anime".to_owned()),
    // ];

    // let book_data = vec![S!(name, Math), S!(artist, John)];
    // println!("{}", res1);

    let res1 = schema_finder(&schemalist, &vec!["root"], &anime_data);
    let res2 = schema_finder(&schemalist, &vec!["root"], &book_data);

    dbg!(&res1, res1.as_ref().unwrap().to_path(&schemalist));
    dbg!(&res2, res2.as_ref().unwrap().to_path(&schemalist));

    // dbg!(schema_finder(
    //     &schemalist,
    //     "Root".to_owned(),
    //     &anime_data,
    //     "Root".to_owned()
    // ).);
    // dbg!()
    // dbg!(schema_finder(
    //     &schemalist,
    //     "Root".to_owned(),
    //     &book_data,
    //     "Root".to_owned()
    // ));
}
