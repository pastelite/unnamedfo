use std::fs;

use crate::{db::IndexDB, path::FilePath};

/// path please start as ./ the working directory is already saved in db
/// if dir is not exists, dir_index is -1
async fn indexer<P: Into<FilePath>>(
    path: P,
    db: &mut IndexDB,
    dir_index: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = path.into();
    let full_path = db.get_path().to_owned() + &path;
    let dir = fs::read_dir(&full_path.as_path())?;

    let db_dir = db.children(dir_index).await?;

    for item in dir {
        let item = item?;
        let item_path = item.path();

        if item_path.is_dir() {
            println!("{:?}", item);
        } else {
            println!("{:?}", item);
            db.add_file(&path, dir_index).await?;
        }
    }
    Ok(())
}

#[async_std::test]
async fn test() {
    let mut db = IndexDB::open("./testdir").await.unwrap();

    indexer(&FilePath::from("/"), &mut db, 0).await.unwrap();
}
