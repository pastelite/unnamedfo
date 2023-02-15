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

use std::{collections::HashMap, path::Path};

use async_std::path::PathBuf;

use crate::{
    config_reader::Config,
    error::FOError,
    format::PatternString,
    schema::{self, SchemaList},
};

struct Move {
    path: PathBuf,
}

impl Move {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn move_file(
        &self,
        config: &Config,
        schemalist: SchemaList,
        schemaname: String,
    ) -> Result<(), FOError> {
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
                    // loop through schema
                    for schema in schemalist
                        .get_children(&schemaname)
                        .ok_or(FOError::SchemaError("NotFound".to_owned()))?
                    {
                        schema.is_fit(&matches);
                    }
                }
                None => {
                    continue;
                }
            }
        }
        Ok(())
    }
}

fn schema_finder(
    schemalist: &SchemaList,
    schemaname: String,
    data: &Vec<(String, String)>,
    schema_path: String,
) -> Option<MoveTree> {
    for schema in schemalist.get_children(&schemaname)? {
        if schema.is_fit(&data) {
            // prune data
            let mut data = data.clone();
            let a = schema.fields.keys().collect::<Vec<&String>>();
            let (data_pruned, data_rest) = data.into_iter().partition(|(k, _)| a.contains(&k));

            let a = schema_finder(
                schemalist,
                schema.name.clone(),
                &data_rest,
                schema_path.clone(),
            );
            match a {
                None => continue,
                Some(a) => {
                    return Some(MoveTree {
                        name: schema.name.clone(),
                        fields: data_pruned,
                        children: Some(Box::new(a)),
                    });
                }
            }
        }
    }
    Some(MoveTree {
        name: "_Uncategorized".to_owned(),
        fields: data.clone(),
        children: None,
    })
}

macro_rules! S {
    ($($var:expr),*) => {
        ($(stringify!($var).to_owned() ),*)
    };
}

#[derive(Debug)]
struct MoveTree {
    name: String,
    fields: Vec<(String, String)>,
    children: Option<Box<MoveTree>>,
}

#[test]
fn finder_test() {
    let mut schemalist = SchemaList::new();
    schemalist.parse_format("Anime".to_owned(), "name epinum(num)! | Video | ");
    schemalist.parse_format("Video".to_owned(), "tags | | ");
    schemalist.parse_format("Books".to_owned(), "name artist | Book | ");
    schemalist.parse_format("Book".to_owned(), "tags | | ");
    schemalist.parse_format("Root".to_owned(), " | Anime Book | ");

    let anime_data = vec![
        ("name".to_owned(), "Naruto".to_owned()),
        ("epinum".to_owned(), "1".to_owned()),
        ("tags".to_owned(), "Anime".to_owned()),
    ];

    let book_data = vec![S!(name, Math), S!(artist, John)];

    dbg!(schema_finder(
        &schemalist,
        "Root".to_owned(),
        &anime_data,
        "Root".to_owned()
    ));
    dbg!(schema_finder(
        &schemalist,
        "Root".to_owned(),
        &book_data,
        "Root".to_owned()
    ));
}
