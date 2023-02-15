use std::{
    cmp::max,
    collections::HashMap,
    ffi::OsStr,
    fs,
    hash::Hash,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};

use crate::{
    db::IndexDB,
    helper::{FileHelper, PathHelper},
    schema::SchemaList,
};

pub struct Indexer<'a> {
    db: &'a mut IndexDB,
    schema: SchemaList,
}

impl<'a> Indexer<'a> {
    pub fn open(db: &'a mut IndexDB) -> Self {
        Self {
            db,
            schema: SchemaList::new(),
        }
    }

    /// path please start as ./ the working directory is already saved in db
    /// if dir is not exists, dir_index is -1
    #[async_recursion::async_recursion]
    pub async fn indexing<P: AsRef<Path> + std::marker::Send>(
        &mut self,
        path: P,
        parent_index: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = PathBuf::from(path.as_ref());
        // let path = path.into();
        dbg!(self.db.get_path_new(), &path);
        let full_path = self.db.get_path_new().join(path);
        dbg!(&full_path);
        // let full_path = self.db.get_path().to_owned() + &path;
        let dir = fs::read_dir(&full_path)?;

        let db_children = self.db.children(parent_index).await?;
        let mut db_children_checker: HashMap<i32, bool> =
            db_children.values().map(|v| (v.id, false)).collect();

        for item in dir {
            dbg!(&item);
            let item = item?;
            let item_full_path = &item.path();
            let file_helper = FileHelper::new(item_full_path);
            let item_cut_path = item_full_path.cut(self.db.get_path_new());
            let file_name = file_helper.file_name();

            // ignore fo.db
            if file_name.eq("fo.db") {
                continue;
            }

            let last_mod = file_helper.last_mod().unwrap_or(DateTime::from(Utc::now()));
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
                        // config related
                        let config = file_helper.read_config()?;
                        for (name, schemaitem) in config.schema.items {
                            self.schema.parse_config(name, &schemaitem);
                            // self.schema.add(schema);
                        }
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

/// Newer indexer. old one still works but I don't like it
/// old one have no schema support, I try to fix that in the new one
// pub async fn indexer_new(path: db, SchemaList) {

// }

#[async_std::test]
async fn test_struct() {
    let mut db = IndexDB::open("./testdir").await.unwrap();
    let mut st = Indexer::open(&mut db);
    st.indexing("./", 0).await.unwrap();
    dbg!(st.schema);
}
