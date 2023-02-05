use std::{
    cmp::max,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};

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
