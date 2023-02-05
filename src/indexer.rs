use std::{
    cmp::max,
    collections::HashMap,
    ffi::OsStr,
    fs,
    hash::Hash,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};

use crate::{db::IndexDB, path::FilePath};

pub struct Indexer<'a> {
    md5_save: HashMap<String, String>,
    db: &'a mut IndexDB,
}

impl<'a> Indexer<'a> {
    pub fn open(db: &'a mut IndexDB) -> Self {
        Self {
            md5_save: HashMap::new(),
            db: db,
        }
    }

    /// path please start as ./ the working directory is already saved in db
    /// if dir is not exists, dir_index is -1
    #[async_recursion::async_recursion]
    pub async fn indexing<P: Into<FilePath> + std::marker::Send>(
        &mut self,
        path: P,
        parent_index: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = path.into();
        let full_path = self.db.get_path().to_owned() + &path;
        let dir = fs::read_dir(&full_path.as_path())?;

        let db_children = self.db.children(parent_index).await?;
        let mut db_children_checker: HashMap<i32, bool> =
            db_children.values().map(|v| (v.id, false)).collect();

        for item in dir {
            dbg!(&item);
            let item = item?;
            let item_full_path = &item.path();
            let file_helper = FileHelper::new(item_full_path);
            let item_cut_path = FilePath::from(item_full_path).cut(self.db.get_path());
            let file_name = file_helper.file_name();

            // ignore fo.db
            if file_name.eq("fo.db") {
                continue;
            }

            // let file_helper = FileHelper::new(item_full_path);
            let last_mod = file_helper.last_mod().unwrap_or(DateTime::from(Utc::now()));

            // let last_mod: DateTime<Utc> = DateTime::from(file_meta.modified().unwrap());
            match db_children.get(&file_name) {
                Some(db_child) if last_mod.timestamp().eq(&db_child.last_modified.timestamp()) => {
                    dbg!("file/folder already exists and not modified");
                    db_children_checker
                        .entry(db_child.id)
                        .and_modify(|d| *d = true);
                    continue;
                }
                Some(db_child) if db_child.is_folder => {
                    dbg!("folder already exists but modified");
                    let dir_id = db_child.id;
                    self.indexing(&item_cut_path, dir_id).await?;
                    db_children_checker
                        .entry(db_child.id)
                        .and_modify(|d| *d = true);
                }
                _ => {
                    dbg!("adding file/folder");

                    if item_full_path.is_dir() {
                        let dir_id = self.db.add_folder(&item_cut_path, parent_index).await?;
                        self.indexing(&item_cut_path, dir_id).await?;
                    } else {
                        self.db.add_file(&item_cut_path, parent_index).await?;
                    }
                }
            }
        }

        // deleted the unused item
        dbg!("deleting...");
        for (key, _) in db_children_checker.iter().filter(|(_, d)| !**d) {
            self.db.delete(*key).await?;
            dbg!(key);
        }

        Ok(())
    }
}

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

    pub fn cut_path<P: AsRef<Path>>(&self, path: P) -> String {
        // let to_cut = FilePath::from(path.as_ref());
        // let orig = FilePath::from(&self.path).cut(&to_cut);
        // orig.as_string()

        let to_cut_path = PathBuf::from(path.as_ref());
        let to_cut_path_filename = to_cut_path.file_name().unwrap_or(OsStr::new(""));
        let mut cut_path = vec![];
        let mut after = false;
        for orig_path in self.path.iter() {
            if orig_path.eq(&to_cut_path_filename.to_owned()) {
                after = true;
            }
            if after {
                cut_path.push(orig_path.to_str().unwrap().to_owned());
            }
        }

        let ret: String = format!("./{}", cut_path.join("/"));
        dbg!(&ret, &cut_path);
        ret
        // let mut i = 0;
        // for path in self.path. {
        //     println!("{} {} {}", path, &paths.data.last().unwrap(), i);
        //     if path.eq(paths.data.last().unwrap()) {
        //         break;
        //     }
        //     i += 1;
        // }
        // self.data.drain(0..(i + 1));
    }

    pub fn trimmed_path(&self) -> String {
        let mut path = self.path.to_str().unwrap_or("//").to_owned();
        path = path.replace("\\", "/");
        path = path.trim_end_matches('/').to_owned();
        path = path.trim_start_matches("./").to_owned();
        path = path.trim_start_matches("/").to_owned();
        // if !path.starts_with("./") {
        //     path = format!("./{}", path);
        // }

        path
    }

    pub fn append(&self, path: &str) -> String {
        // if !path.starts_with("./") {
        //     path = format!("./{}", path);
        // }
        format!(
            "{}/{}",
            self.trimmed_path(),
            FileHelper::new(path).trimmed_path()
        )
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

#[async_std::test]
async fn test_struct() {
    let mut db = IndexDB::open("./testdir").await.unwrap();
    let mut st = Indexer::open(&mut db);
    st.indexing("/", 0).await.unwrap();
    dbg!(st.md5_save);
}

#[test]
fn helper() {
    let path = FileHelper::new(".\\testdir/wew/f/tess");
    dbg!(path.trimmed_path());
    dbg!(path.cut_path(".\\testdir/wew"));
}
