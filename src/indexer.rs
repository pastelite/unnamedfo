use std::{collections::HashMap, fs};

use chrono::{DateTime, Utc};

use crate::{db::IndexDB, path::FilePath};

/// path please start as ./ the working directory is already saved in db
/// if dir is not exists, dir_index is -1
#[async_recursion::async_recursion]
pub async fn indexer<P: Into<FilePath> + std::marker::Send>(
    path: P,
    db: &mut IndexDB,
    dir_index: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = path.into();
    let full_path = db.get_path().to_owned() + &path;
    let dir = fs::read_dir(&full_path.as_path())?;

    let db_children = db.children(dir_index).await?;
    let mut db_children_ticker: HashMap<String, bool> =
        db_children.keys().map(|k| (k.to_owned(), false)).collect();

    // adding items to db
    for item in dir {
        dbg!(&item);
        let item = item?;
        let item_full_path = &item.path();
        let item_cut_path = FilePath::from(item_full_path).cut(&db.get_path());
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
                db_children_ticker
                    .entry(file_name.to_string())
                    .and_modify(|d| *d = true);
                continue;
            }
            Some(db_child) if db_child.is_folder => {
                dbg!("folder already exists but modified");
                let dir_id = db_child.id;
                indexer(&item_cut_path, db, dir_id).await?;
                db_children_ticker
                    .entry(file_name.to_string())
                    .and_modify(|d| *d = true);
            }
            _ => {
                dbg!("adding file/folder");

                if item_full_path.is_dir() {
                    let dir_id = db.add_folder(&item_cut_path, dir_index).await?;
                    indexer(&item_cut_path, db, dir_id).await?;
                } else {
                    db.add_file(&item_cut_path, dir_index).await?;
                }
            }
        }
    }

    // deleted the unused item
    dbg!("deleting...");
    for (key, data) in db_children_ticker.iter().filter(|(_, d)| !**d) {
        let item = db_children.get(key).unwrap();
        db.delete(item.id).await?;
        dbg!(key);
    }
    Ok(())
}

#[async_std::test]
async fn test() {
    let mut db = IndexDB::open("./testdir").await.unwrap();

    indexer(&FilePath::from("/"), &mut db, 0).await.unwrap();
}
