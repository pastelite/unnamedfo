use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use db::IndexDB;
use rusqlite::Connection;

fn default_path() -> PathBuf {
    return PathBuf::from("./testdir");
}

mod db;

fn good_file_name(filename: &OsStr) -> bool {
    filename != "fo.db"
}

fn indexer(path: &Path, db: &mut IndexDB) -> Result<(), Box<dyn Error>> {
    let ignore_file_list = [OsStr::new("fo.db")];

    let dir = fs::read_dir(path)?;

    for file in dir {
        let path = file.unwrap().path();
        if path.is_dir() {
            indexer(&path, db)?;
        }
        if path.is_file() {
            let file_name = path.file_name();
            println!("{:?}", path.file_name().unwrap());
            if good_file_name(file_name.unwrap()) {
                db.add_file(&path).unwrap();
                println!("Name: {:?}", path)
            }
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut path = default_path();
    let dir = fs::read_dir(&path)?;

    path.push("./fo.db");
    let mut db = IndexDB::open(&path)?;
    db.setup()?;
    path.pop();

    indexer(&path, &mut db).unwrap();

    return Ok(());
}
