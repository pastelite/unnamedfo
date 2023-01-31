use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use db::IndexDB;
use rusqlite::Connection;

fn default_path() -> PathBuf {
    return PathBuf::from("./testdir");
}

mod db;

fn main() -> Result<(), Box<dyn Error>> {
    let mut path = default_path();
    let dir = fs::read_dir(&path)?;

    path.push("./fo.db");
    let db = IndexDB::open(&path)?;
    db.setup()?;
    path.pop();

    for path in dir {
        let path = path.unwrap().path();
        if !path.ends_with("fo.db") {
            println!("Name: {}", path.display())
        }
    }

    return Ok(());
}
