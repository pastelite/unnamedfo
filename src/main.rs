use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use async_recursion::async_recursion;
use clap::Parser;
use db::{File, IndexDB};

use crate::path::FilePath;
// use rusqlite::Connection;

fn default_path() -> PathBuf {
    return PathBuf::from("./testdir");
}

mod db;
mod path;

fn good_file_name(filename: &OsStr) -> bool {
    filename != "fo.db"
}

#[async_recursion]
async fn indexer(path: &Path, cutpath: &FilePath, db: &mut IndexDB) -> Result<(), Box<dyn Error>> {
    let ignore_file_list = [OsStr::new("fo.db")];

    let dir = fs::read_dir(path)?;

    for file in dir {
        let path = file.unwrap().path();
        if path.is_dir() {
            indexer(&path, cutpath, db).await?;
        }
        if path.is_file() {
            let file_name = path.file_name();
            println!("{:?}", path.file_name().unwrap());
            if good_file_name(file_name.unwrap()) {
                db.add_file(FilePath::from(&path).cut(cutpath)).await?;
                println!("Name: {:?}", path)
            }
        }
    }

    Ok(())
}

async fn search(db: &IndexDB, text: &str) -> Vec<File> {
    let data = db.search(text).await.unwrap();
    return data;
}

#[derive(Parser, Debug)]
struct CliArgs {
    search: Vec<String>,
    #[arg(short, long, default_value = "./")]
    path: String,
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = CliArgs::parse();

    if cfg!(debug_assertions) {
        args.path = "./testdir".to_string();
    }

    println!("{:#?}", args);

    let path = FilePath::from(args.path.as_str());
    let is_exist = path.exists();

    let mut db = IndexDB::open(&path.get_path()).await?;

    // set up
    println!("{}", is_exist);
    // if !is_exist {
    //     println!("first time setup...");
    db.setup().await?;
    indexer(
        &PathBuf::from(&args.path),
        &FilePath::from(&PathBuf::from(&args.path)),
        &mut db,
    )
    .await?;
    // }

    // search
    if args.search.len() > 0 {
        let files = search(&db, &args.search.join(" ")).await;
        println!("{:#?}", files);
    }

    // let mut path = default_path();
    // let dir = fs::read_dir(&path)?;

    // path.push("./fo.db");
    // let mut db = IndexDB::open(path.to_str().unwrap()).await?;
    // db.setup().await?;
    // path.pop();

    // indexer(&path, &FilePath::from(&path), &mut db).await?;

    // return Ok(());
    Ok(())
}

#[async_std::test]
async fn test_search() -> Result<(), Box<dyn Error>> {
    let mut db = IndexDB::new().await?;
    let files = search(&db, "txt").await;
    println!("{:#?}", files);
    Ok(())
}
