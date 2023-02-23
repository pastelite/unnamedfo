use std::{
    cmp::max,
    collections::HashMap,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};

use crate::{
    config_reader::{Config, SchemaConfig},
    error::FOError,
};

pub fn cut_path<P: AsRef<Path>, Q: AsRef<Path>>(full: P, cut: Q) -> PathBuf {
    let cut_path = PathBuf::from(cut.as_ref());
    let full_path = PathBuf::from(full.as_ref());
    let cut_path_fn = cut_path.file_name().unwrap_or(OsStr::new(""));

    let mut cutted = vec![];
    let mut after = false;
    for path_item in full_path.iter() {
        if path_item.eq(cut_path_fn) {
            after = true;
            continue;
        }
        if after {
            cutted.push(path_item.to_str().unwrap().to_owned());
        }
    }
    PathBuf::from(format!("./{}", cutted.join("/")))
}
pub trait PathHelper {
    fn to_string(&self) -> String;
    fn format(&self) -> String;
    fn cut<P: AsRef<Path>>(&self, cut: P) -> PathBuf;
}

pub trait StringHelper {
    fn to_path(&self) -> PathBuf;
    fn to_string(&self) -> String;
}

macro_rules! path_helper {
    ($($t:ty),*) => {
        $(impl PathHelper for $t {
            fn to_string(&self) -> String {
                self.to_str().unwrap().to_owned()
            }

            fn format(&self) -> String {
                self.iter()
                    .map(|p| p.to_str().unwrap())
                    .collect::<Vec<&str>>()
                    .join("/")
            }

            fn cut<P: AsRef<Path>>(&self, cut: P) -> PathBuf {
                cut_path(self, cut)
            }
        })*
    };
}

macro_rules! string_helper {
    ($($t:ty),*) => {
        $(impl StringHelper for $t {
            fn to_path(&self) -> PathBuf {
                PathBuf::from(self)
            }

            fn to_string(&self) -> String {
                self.to_str().unwrap().to_owned()
            }
        })*
    };
}

path_helper!(Path, PathBuf, &Path, &PathBuf);
string_helper!(OsStr, &OsStr);

pub struct FileHelper {
    path: PathBuf,
}

impl FileHelper {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_owned(),
        }
    }

    pub fn file_name(&self) -> String {
        self.path.file_name().unwrap().to_str().unwrap().to_owned()
    }

    pub fn last_mod(&self) -> Option<DateTime<Utc>> {
        let file_meta = fs::metadata(&self.path).ok();
        let yaml_path = self.path.with_extension("yaml");
        let yaml_meta = fs::metadata(&yaml_path).ok();

        let get_dt = |d: fs::Metadata| {
            let date: DateTime<Utc> = DateTime::from(d.modified().ok()?);
            Some(date)
        };

        match (file_meta, yaml_meta) {
            (Some(d1), Some(d2)) => Some(max(get_dt(d1)?, get_dt(d2)?)),
            (Some(d), None) | (None, Some(d)) => Some(get_dt(d)?),
            _ => None,
        }
    }

    pub fn get_path(&self) -> &Path {
        &self.path
    }

    pub fn read_config(&self) -> Result<Config, FOError> {
        let mut config: Config = Default::default();

        // TODO: add other stuff

        // yaml outside
        read_file_to_combine_config(&mut config, &self.path.with_extension("yaml"), |yaml| {
            serde_yaml::from_str::<Config>(yaml).map_err(|e| e.into())
        })?;
        read_file_to_combine_config(
            &mut config,
            &self.path.with_extension("schema.yaml"),
            |yaml| {
                let schema = serde_yaml::from_str::<SchemaConfig>(yaml)?;
                // parse .schema.yaml as schema
                Ok(Config {
                    schema,
                    ..Default::default()
                })
            },
        )?;

        // yaml inside
        read_file_to_combine_config(&mut config, &self.path.join("_data.yaml"), |yaml| {
            serde_yaml::from_str::<Config>(yaml).map_err(|e| e.into())
        })?;

        Ok(config)
    }

    pub fn read_dir(&self) -> Result<Vec<FileHelper>, FOError> {
        let mut files = vec![];
        for entry in fs::read_dir(&self.path)? {
            let entry = entry?;
            let path = entry.path();
            files.push(FileHelper::new(path));
            // if path.is_dir() {
            //     files.push(FileHelper::new(path));
            // }
        }
        Ok(files)
    }
}

fn read_file_to_combine_config<F>(
    config: &mut Config,
    path: &Path,
    config_dealer: F,
) -> Result<(), FOError>
where
    F: FnOnce(&str) -> Result<Config, FOError>,
{
    let yaml = fs::read_to_string(path);
    match yaml {
        Ok(yaml) => {
            let config_new = config_dealer(&yaml)?;
            config.combine_config(&config_new, false);
        }
        Err(e) if !matches!(e.kind(), std::io::ErrorKind::NotFound) => return Err(e.into()),
        _ => {}
    }
    // let config: Config = serde_yaml::from_str(&yaml)?;
    Ok(())
}

#[test]
fn test_cutter() {
    assert_eq!(
        cut_path(
            "./testdir/1/2/3/4/5/6/7/8/9/10",
            "./testdir/1/2/3/4/5/6/7/8/9"
        ),
        PathBuf::from("./10")
    );
    assert_eq!(
        cut_path("./testdir\\1/2/3/4", "1"),
        PathBuf::from("./2/3/4")
    );
    assert_eq!(
        PathBuf::from("./testdir").join("./data1"),
        PathBuf::from("./testdir/data1")
    );
    dbg!(PathBuf::from("C:/document/s\\data1").format());
}

#[test]
fn test_read_config() {
    let file_helper = FileHelper::new("./testdir/test2");
    let config = file_helper.read_config().unwrap();
    dbg!(config);
    let file_helper = FileHelper::new("./testdir/test.txt");
    let config = file_helper.read_config().unwrap();
    dbg!(config);
}

pub fn match_text(full: &str, short: &str) -> bool {
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

#[derive(Debug)]
pub struct FieldHashMapBuilder {
    // field, (schema_notation, data)
    list: HashMap<String, (String, String)>,
    schema_name: String,
}

impl FieldHashMapBuilder {
    pub fn new<T: AsRef<str>>(name: T) -> Self {
        Self {
            list: HashMap::new(),
            schema_name: String::from(name.as_ref()),
        }
    }
    pub fn insert(mut self, data: &Vec<(String, String)>) -> Self {
        // let mut map = HashMap::new();
        for (field, data) in data {
            let (schema_notation, field) = if field.contains(".") {
                let mut split = field.split(".");
                let schema_notation = split.next().unwrap();
                let field = split.collect::<Vec<&str>>().join(".");
                (schema_notation.to_owned(), field)
            } else {
                ("".to_owned(), field.to_owned())
            };
            // let schema_notation = field.replace(".", "_");
            if (match_text(&schema_notation, &self.schema_name)) {
                self.list
                    .entry(field.to_owned())
                    .and_modify(|(item_schema_notation, item_data)| {
                        if schema_notation.len() > item_schema_notation.len() {
                            *item_schema_notation = schema_notation.to_owned();
                            *item_data = data.to_owned();
                        }
                    })
                    .or_insert((schema_notation, data.to_owned()));
            }
            // self.list
            //     .insert(field.to_owned(), (schema_notation, data.to_owned()));
        }
        self
        // Self { list: map }
    }

    pub fn to_map(self) -> HashMap<String, String> {
        self.list
            .iter()
            .map(|(field, (_, data))| (field.to_owned(), data.to_owned()))
            .collect()
    }
}

// impl From<&Vec<(String, String)>> for FieldDotListHelper {
//     fn from(list: &Vec<(String, String)>) -> Self {
//         let mut map = HashMap::new();
//         for (field, data) in list {
//             let (schema_notation, field) = if field.contains(".") {
//                 let mut split = field.split(".");
//                 let schema_notation = split.next().unwrap();
//                 let field = split.collect::<Vec<&str>>().join(".");
//                 (schema_notation.to_owned(), field)
//             } else {
//                 ("".to_owned(), field.to_owned())
//             };
//             // let schema_notation = field.replace(".", "_");
//             map.insert(field.to_owned(), (schema_notation, data.to_owned()));
//         }
//         Self { list: map }
//     }
// }

#[test]
fn test_field_dot_list() {
    let data = vec![
        ("a.b".to_owned(), "1".to_owned()),
        ("a.c".to_owned(), "2".to_owned()),
        ("a.d".to_owned(), "3".to_owned()),
        ("b".to_owned(), "4".to_owned()),
        ("ab.d".to_owned(), "5".to_owned()),
        ("abn.d".to_owned(), "6".to_owned()),
    ];

    let mut helper = FieldHashMapBuilder::new("abc").insert(&data);
    dbg!(helper.to_map());
}
