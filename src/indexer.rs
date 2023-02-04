use std::{collections::HashMap, fs, hash::Hash};

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
            let item_cut_path = FilePath::from(item_full_path).cut(self.db.get_path());
            let file_name = FilePath::from(item_full_path).get_name();
            let file_meta = item.metadata()?;

            // ignore fo.db
            if file_name.eq("fo.db") {
                continue;
            }

            let last_mod: DateTime<Utc> = DateTime::from(file_meta.modified().unwrap());
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

#[async_std::test]
async fn test_struct() {
    let mut db = IndexDB::open("./testdir").await.unwrap();
    let mut st = Indexer::open(&mut db);
    st.indexing("/", 0).await.unwrap();
    dbg!(st.md5_save);
}
