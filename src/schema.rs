use std::{
    borrow::BorrowMut,
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    rc::Rc,
    sync::Arc,
};

use regex::{Captures, Regex};

use crate::config_reader::{SchemaConfig, SchemaConfigItem};

#[derive(Clone, Debug)]
pub struct Schema {
    pub name: String,
    pub fields: Vec<Field>,
    pub children: Vec<String>,
    pub filename: Option<String>,
}

impl Schema {
    pub fn to_format(&self) -> String {
        let mut string = String::new();
        string.push_str(
            &self
                .fields
                .iter()
                .map(|field| field.to_format())
                .collect::<Vec<String>>()
                .join(" "),
        );
        string.push('|');
        string.push_str(&self.children.join(" "));
        string.push('|');
        if let Some(filename) = &self.filename {
            string.push_str(&filename);
        }
        string

        // for field in &self.fields {
        //     string.push_str(&field.to_format());
        //     string.push_str(" ");
        // }
    }
}

#[derive(Debug, Clone)]
pub struct Field {
    name: String,
    format: FieldFormat,
    forced: bool,
}

impl Field {
    pub fn to_format(&self) -> String {
        let mut string = String::from(&self.name);
        match self.format {
            FieldFormat::String => (),
            _ => string.push_str(&format!("({})", self.format.to_string())),
        }
        if self.forced {
            string.push_str("!");
        }
        string
    }

    pub fn from_format(format: &str) -> Option<Self> {
        let re = Regex::new(r"^(\w+)(?:\((\w+)\))?(!)?$").unwrap();
        let captures = re.captures(format).unwrap();
        Some(Self {
            name: captures.get(1)?.as_str().to_string(),
            format: FieldFormat::parse(match captures.get(2) {
                Some(d) => d.as_str(),
                None => "str",
            }),
            forced: captures.get(3).is_some(),
        })
    }
}

// impl Debug for Schema {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.write_str(&self.to_format())
//         // Ok(self.to_format())
//         // f.
//         // f.debug_struct("Schema")
//         //     .field("formatted", &self.to_format())
//         //     // .field("name", &self.name)
//         //     // .field("fields", &self.fields)
//         //     // .field("children", &self.children)
//         //     // .field("filename", &self.filename)
//         //     .finish()
//     }
// }

impl Schema {
    pub fn new(name: String) -> Self {
        Self {
            name,
            fields: Vec::new(),
            children: Vec::new(),
            filename: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum FieldFormat {
    String,
    Number,
    StringArray,
}

impl FieldFormat {
    pub fn parse(format: &str) -> Self {
        match format {
            "num" => Self::Number,
            "str" => Self::String,
            "str[]" => Self::StringArray,
            _ => Self::String,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Self::Number => "num",
            Self::String => "str",
            Self::StringArray => "str[]",
        }
        .to_string()
    }
}

#[derive(Debug)]
pub struct SchemaList {
    pub list: HashMap<String, Rc<RefCell<Schema>>>,
}

impl SchemaList {
    // field format:
    // field1 field2! field3(num) | child1, child2 | filename
    // filename format:
    // %field1%.%field2%

    pub fn new() -> Self {
        Self {
            list: HashMap::new(),
        }
    }

    pub fn parse_format(&mut self, name: String, format: &str) -> Option<()> {
        let mut format = format.split("|");
        let re = Regex::new(r"^(\w+)(?:\((\w+)\))?(!)?$").unwrap();

        // parse Fields
        let fields: Vec<Field> = format
            .next()?
            .split(" ")
            .filter_map(|f| {
                Some(Field::from_format(f)?)
                // let captures = re.captures(f)?;
                // Some(Field {
                //     name: captures.get(1)?.as_str().to_string(),
                //     format: FieldFormat::parse(match captures.get(2) {
                //         Some(d) => d.as_str(),
                //         None => "str",
                //     }),
                //     forced: captures.get(3).is_some(),
                // })
            })
            .collect();

        let children: Vec<String> = format
            .next()?
            .split(" ")
            .filter_map(|c| {
                if c.eq("") {
                    return None;
                }

                if !self.list.contains_key(c) {
                    self.insert_empty(c.to_owned());
                }

                Some(c.to_owned())
            })
            .collect();

        // parse filename
        let filename = format.next().and_then(|s| {
            if s.trim().eq("") {
                return None;
            }
            Some(s.trim().to_string())
        });

        let schema = Schema {
            name: name.clone(),
            fields,
            children,
            filename,
        };

        self.list
            .entry(name.clone())
            .and_modify(|f| {
                f.borrow_mut().replace(schema.clone());
            })
            .or_insert(Rc::new(RefCell::new(schema)));
        Some(())
    }

    pub fn parse_config(&mut self, name: String, config: &SchemaConfigItem) -> Option<()> {
        let fields = config
            .fields
            .0
            .iter()
            .filter_map(|f| Some(Field::from_format(f)?))
            .collect::<Vec<Field>>();

        let children = config
            .children
            .0
            .iter()
            .map(|c| {
                if !self.list.contains_key(c) {
                    self.insert_empty(c.clone());
                }
                name.clone()
            })
            .collect::<Vec<String>>();

        let filename = config.filename.clone();

        let schema = Schema {
            name: name.clone(),
            fields,
            children,
            filename,
        };

        self.list
            .entry(name)
            .and_modify(|f| {
                f.borrow_mut().replace(schema.clone());
            })
            .or_insert(Rc::new(RefCell::new(schema)));

        Some(())
    }

    fn insert_empty(&mut self, name: String) {
        self.list
            .insert(name.clone(), Rc::new(RefCell::new(Schema::new(name))));
    }

    fn insert(&mut self, schema: Rc<RefCell<Schema>>) {
        self.list
            .insert(schema.borrow().name.to_owned(), schema.clone());
    }
}

impl From<&SchemaConfig> for SchemaList {
    fn from(config: &SchemaConfig) -> Self {
        let mut sl = SchemaList::new();
        for (name, config) in config.items.iter() {
            sl.parse_config(name.to_owned(), config);
        }
        sl
    }
}

#[test]
fn test_parse() {
    let mut sl = SchemaList::new();
    sl.parse_format(
        "any".to_owned(),
        "field1 field2! field3(num) | child1 child2 | filename",
    );
    sl.parse_format("custom".to_owned(), "field1 field2! field3(num) | any |");
    sl.parse_format("child1".to_owned(), "testf| |");
    // sl.parse_format("child2".to_owned(), "fs(num)!| |");
    dbg!(&sl.list);
    sl.list.iter().for_each(|(_, rc)| {
        dbg!(rc.borrow().to_format());
    })
}

enum FormatTree {
    Folder {
        schema: Rc<Schema>,
        fields: HashMap<String, String>,
        children: Box<FormatTree>,
    },
    File {
        filename: String,
        fields: HashMap<String, String>,
        current_path: String,
    },
}
