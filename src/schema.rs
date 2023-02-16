use std::{
    borrow::BorrowMut,
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    mem::replace,
    rc::Rc,
    sync::Arc,
};

use regex::{Captures, Regex};

use crate::{
    config_reader::{CommaSeperated, ConfigDatatype, SchemaConfig, SchemaConfigItem},
    format::FormatString,
};

#[derive(Clone, Debug)]
pub struct Schema {
    pub name: String,
    pub fields: HashMap<String, Field>,
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
                .map(|field| field.1.to_format())
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
    format: ConfigDatatype,
    forced: bool,
}

impl Field {
    pub fn to_format(&self) -> String {
        let mut string = String::from(&self.name);
        match self.format {
            ConfigDatatype::String(_) => (),
            _ => string.push_str(&format!("({})", self.format.to_string())),
        }
        if self.forced {
            string.push_str("!");
        }
        string
    }

    pub fn from_format(format: &str) -> Option<Self> {
        let re = Regex::new(r"^(\w+)(?:\((\w+)\))?(!)?$").unwrap();
        let captures = re.captures(format)?;
        Some(Self {
            name: captures.get(1)?.as_str().to_string(),
            format: ConfigDatatype::parse(match captures.get(2) {
                Some(d) => d.as_str(),
                None => "str",
            }),
            forced: captures.get(3).is_some(),
        })
    }

    pub fn is_fit(&self, field: &String, data: &String) -> bool {
        match (&self.format, data) {
            (&ConfigDatatype::String(_), _) => (),
            (&ConfigDatatype::Integer(_), s) => {
                if s.parse::<i32>().is_err() {
                    return false;
                }
            }
            (&ConfigDatatype::Float(_), s) => {
                if s.parse::<f32>().is_err() {
                    return false;
                }
            }
            (&ConfigDatatype::Tags(_), s) => {
                let b = serde_yaml::from_str::<CommaSeperated>(s);
                if b.is_err() {
                    return false;
                }
            }
            _ => return false,
        }
        self.name.eq(field)
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
            fields: HashMap::new(),
            children: Vec::new(),
            filename: None,
        }
    }

    pub fn is_fit(&self, data: &Vec<(String, String)>) -> bool {
        let data_map = data.iter().map(|d| (&d.0, &d.1)).collect::<HashMap<_, _>>();
        for (field_name, field_type) in &self.fields {
            // if data_map.get(field_name).is_none() && !field_type.forced {
            //     return false;
            // }
            match data_map.get(field_name) {
                None if field_type.forced => return false,
                None => continue,
                Some(d) if !field_type.is_fit(field_name, d) => {
                    return false;
                }
                _ => continue,
            }
        }
        true
    }

    pub fn generate_string(&self, data: &Vec<(String, String)>) -> String {
        let data_map = data.iter().map(|d| (&d.0, &d.1)).collect::<HashMap<_, _>>();

        let filename_formatter = FormatString::parse(&self.filename.as_ref().unwrap());
        let vars = filename_formatter
            .vars
            .iter()
            .map(|var| match data_map.get(var) {
                None => String::new(),
                Some(d) => d.to_string(),
            })
            .collect::<Vec<String>>();
        let string = filename_formatter.generate_string(&vars);
        string
    }
}

// #[derive(Debug, Clone)]
// pub enum FieldFormat {
//     String,
//     Number,
//     StringArray,
// }

impl ConfigDatatype {
    pub fn parse(format: &str) -> Self {
        match format {
            "num" => Self::Integer(0),
            "flo" => Self::Float(0.),
            "str" => Self::String("".to_owned()),
            "str[]" => Self::Tags(Vec::new()),
            _ => Self::String("".to_owned()),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Self::Integer(_) => "num",
            Self::Float(_) => "flo",
            Self::String(_) => "str",
            Self::Tags(_) => "str[]",
        }
        .to_string()
    }
}

#[derive(Debug)]
pub struct SchemaList {
    pub list: HashMap<String, Schema>,
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
        let fields: HashMap<String, Field> = format
            .next()?
            .split(" ")
            .filter_map(|f| {
                let field = Field::from_format(f)?;
                Some((field.name.clone(), field))
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
                let _ = replace(f, schema.clone());
                // f = schema.clone();
                // f.borrow_mut() .replace(schema.clone());
            })
            .or_insert(schema);
        Some(())
    }

    pub fn parse_config_item(&mut self, name: String, config: &SchemaConfigItem) -> Option<()> {
        let fields = config
            .fields
            .0
            .iter()
            .filter_map(|f| {
                let field = Field::from_format(f)?;
                Some((field.name.clone(), field))
            })
            .collect::<HashMap<String, Field>>();

        let children = config
            .children
            .0
            .iter()
            .map(|c| {
                if !self.list.contains_key(c) {
                    self.insert_empty(c.clone());
                }
                c.clone()
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
                let _ = replace(f, schema.clone());
                // f = &mut schema.clone();
                // f.borrow_mut().replace(schema.clone());
            })
            .or_insert(schema);

        Some(())
    }

    // pub fn parse_config(&mut self)

    fn insert_empty(&mut self, name: String) {
        self.list.insert(name.clone(), Schema::new(name));
    }

    fn insert(&mut self, schema: Schema) {
        self.list.insert(schema.name.to_owned(), schema.clone());
    }

    /// None if name is not exists
    pub fn get_children(&self, name: &str) -> Option<Vec<&Schema>> {
        let ty = self
            .list
            .get(name)?
            .children
            .iter()
            .flat_map(|f| Some(self.list.get(f)?))
            .collect::<Vec<&Schema>>();
        Some(ty)
    }
}

impl From<&SchemaConfig> for SchemaList {
    fn from(config: &SchemaConfig) -> Self {
        let mut sl = SchemaList::new();
        for (name, config) in config.items.iter() {
            sl.parse_config_item(name.to_owned(), config);
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
    sl.parse_format("child2".to_owned(), "fs(num)!| |");
    dbg!(&sl.list);
    // sl.list.iter().for_each(|(_, rc)| {
    //     dbg!(rc.to_format());
    // })
}

macro_rules! S {
    ($($var:expr),*) => {
        ($(stringify!($var).to_owned() ),*)
    };
}

// macro_rules! vecString {

// }

#[test]
fn fit_test() {
    let mut sl = SchemaList::new();
    sl.parse_format("test1".to_owned(), "a(num) b c | |");
    sl.parse_format("test2".to_owned(), "a b(num) c! | test1 |");

    let test1 = sl.list.get("test1").unwrap();
    let test2 = sl.list.get("test2").unwrap();

    let data_test1 = vec![S!(a, 2), S!(b, test)];
    let data_test2 = vec![S!(a, s), S!(b, 2), S!(c, a)];
    let data_test2_no_c = vec![S!(a, s), S!(b, 2)];

    assert_eq!(true, test1.is_fit(&data_test1));
    assert_eq!(false, test2.is_fit(&data_test1));
    assert_eq!(false, test1.is_fit(&data_test2));
    assert_eq!(true, test2.is_fit(&data_test2));
    assert_eq!(false, test1.is_fit(&data_test2_no_c));
    assert_eq!(false, test2.is_fit(&data_test2_no_c));
}

#[test]
fn generate_string_test() {
    let mut sl = SchemaList::new();
    sl.parse_format("test1".to_owned(), "a b c | | %a%.txt");
    // sl.parse_format("test2".to_owned(), "a b(num) c! | test1 |");
    let test = sl.list.get("test1").unwrap();
    let data = vec![S!(a, 2), S!(b, test)];
    dbg!(test.generate_string(&data));

    // assert_eq!(test1.generate_string(&data_test1), "2 test");
    // assert_eq!(test2.generate_string(&data_test2), "s 2 a");
    // assert_eq!(test2.generate_string(&data_test2_no_c), "s 2");
}
