use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use async_std::stream::StreamExt;
use base64ct::{Base64, Encoding};
use chrono::{DateTime, Local, Utc};
use md5::{Digest, Md5};
use sqlx::sqlite::{SqlitePoolOptions, SqliteRow};
use sqlx::{query, sqlite::SqliteConnectOptions, Executor, Pool, Result, Sqlite, SqlitePool};
use sqlx::{ConnectOptions, Row, SqliteConnection};
use std::fs::{metadata, File};

use crate::helper::{FileHelper, PathHelper};

// use rusqlite::{Connection, ErrorCode};

pub struct IndexDB {
    pool: Pool<Sqlite>,
    path: PathBuf,
}

#[derive(Debug)]
pub struct ChildItem {
    pub id: i32,
    pub path: String,
    pub last_modified: DateTime<Utc>,
    pub md5: Option<String>,
    pub is_folder: bool,
}

// id, other data
type ChildrenList = HashMap<String, ChildItem>;

impl IndexDB {
    pub async fn new() -> Result<Self> {
        let a = SqliteConnectOptions::from_str("sqlite://fo.db")?
            .read_only(false)
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(a).await?;
        Ok(Self {
            pool,
            path: PathBuf::from("./fo.db"),
        })
    }

    /// Open a database file
    /// note: use a path to directory not ./fo.db
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut path = PathBuf::from(path.as_ref());
        path.join("fo.db");
        let db_exists = path.exists();

        // connection
        let db_path = "sqlite://".to_owned() + path.to_str().unwrap();
        println!("connecting to {}", &db_path);
        let a = SqliteConnectOptions::from_str(&db_path)?
            .create_if_missing(true)
            .read_only(false);

        let pool = SqlitePool::connect_with(a).await?;

        // setup db if not exists
        path.pop();
        let mut db = Self { pool, path };
        if !db_exists {
            db.setup().await?;
        }
        Ok(db)
    }

    pub async fn setup(&mut self) -> Result<()> {
        dbg!("setting up the database...");
        // "typeList" table
        query(
            "CREATE TABLE IF NOT EXISTS typeList (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            name  TEXT NOT NULL
        );",
        )
        .execute(&self.pool)
        .await?;

        // "file" table
        dbg!("file table");
        query(
            "CREATE TABLE IF NOT EXISTS files (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            path        TEXT NOT NULL,
            last_mod    TEXT NOT NULL,
            is_folder   INTEGER NOT NULL DEFAULT 0,
            type        INTEGER DEFAULT 0,
            parent      INTEGER,
            FOREIGN KEY (parent) REFERENCES files(id),
            FOREIGN KEY (type) REFERENCES typeList(id),
        );",
        )
        .execute(&self.pool)
        .await?;

        // add typelist
        dbg!("add typelist");
        query("INSERT OR REPLACE INTO typeList VALUES (0,\"any\");")
            .execute(&self.pool)
            .await?;

        // add folder file
        dbg!("add folder");
        query("INSERT OR REPLACE INTO files (id,name,path,last_mod,is_folder) VALUES (0,\"root\",\"/\",datetime('now'),1);")
            .execute(&self.pool)
            .await?;

        // "any" table
        dbg!("any");
        query(
            "CREATE TABLE IF NOT EXISTS any (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            field       TEXT NOT NULL,
            field_value TEXT NOT NULL,
            FOREIGN KEY (id) REFERENCES files(id)
        );",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn add_file<P: AsRef<Path>>(&self, path: P, parent: i32) -> Result<()> {
        // get file
        dbg!("getfile");
        let path = PathBuf::from(path.as_ref());
        let full_path = self.get_path_new().join(&path);

        // get hash
        // dbg!("hash");
        // let mut hasher = md5::Md5::default();
        // std::io::copy(&mut file, &mut hasher)?;
        // let hash = Base64::encode_string(&hasher.finalize());

        // get last mod
        dbg!("lastmod");
        let last_mod = FileHelper::new(&full_path)
            .last_mod()
            .unwrap_or(DateTime::from(Utc::now()));

        dbg!(&last_mod);

        // insert
        dbg!("insert");
        query("INSERT OR REPLACE INTO files(path,name,last_modparent) VALUES (?,?,?,?)")
            .bind(path.format())
            .bind(path.file_name().unwrap().to_str().unwrap())
            .bind(last_mod.naive_utc().format("%Y-%m-%d %H:%M:%S").to_string())
            // .bind(hash)
            .bind(parent)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn add_folder<P: AsRef<Path>>(&self, path: P, parent: i32) -> Result<i32> {
        // get folder
        let path = PathBuf::from(path.as_ref());
        let full_path = self.get_path_new().join(&path);

        // get last mod
        let last_mod = FileHelper::new(&full_path)
            .last_mod()
            .unwrap_or(DateTime::from(Utc::now()));

        // insert
        dbg!("folder insert");
        let result = query(
            "INSERT OR REPLACE INTO files(path,name,last_mod,parent,is_folder) VALUES (?,?,?,?,1)",
        )
        .bind(path.format())
        .bind(path.file_name().unwrap().to_str().unwrap())
        .bind(last_mod.naive_utc().format("%Y-%m-%d %H:%M:%S").to_string())
        .bind(parent)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid() as i32)
    }

    pub async fn delete(&self, id: i32) -> Result<()> {
        // delete from file
        query("DELETE FROM files WHERE id = ?")
            .bind(&id)
            .execute(&self.pool)
            .await?;

        // delete from any
        query("DELETE FROM any WHERE id = ?")
            .bind(&id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // pub async fn search(&self, keyword: &str) -> Result<Vec<File>> {
    //     let mut data = query("SELECT * FROM files WHERE path LIKE ?")
    //         .bind(format!("{}{}{}", "%", keyword, "%"))
    //         // .map(|row|{
    //         //     row.
    //         // })
    //         .fetch_all(&self.pool)
    //         .await?
    //         .iter()
    //         .map(|row| File {
    //             name: row.get::<String, &str>("name"),
    //             path: row.get::<String, &str>("path"),
    //         })
    //         .collect();
    //     Ok(data)

    //     // to add other data
    // }

    pub async fn children(&self, id: i32) -> Result<ChildrenList> {
        let data: ChildrenList = query("SELECT * FROM files WHERE parent = ?")
            .bind(id)
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(|row| {
                (
                    row.get::<String, &str>("name"),
                    ChildItem {
                        id: row.get::<i32, &str>("id"),
                        path: row.get::<String, &str>("path"),
                        last_modified: row.get::<DateTime<Utc>, &str>("last_mod"),
                        md5: (!row.get::<String, &str>("md5").eq(""))
                            .then(|| row.get::<String, &str>("md5")),
                        is_folder: row.get::<bool, &str>("is_folder"),
                    },
                )
            })
            .collect();
        Ok(data)
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn get_path_new(&self) -> PathBuf {
        PathBuf::from(&self.path)
        // (&self.path).to_owned().as_path()
    }
}

#[async_std::test]
async fn db_test() {
    //"./testdir/fo.db"
    let mut db = IndexDB::open("./testdir").await.unwrap();
    db.add_file("./tests.txt", 0).await.unwrap();
    // db.setup().await.unwrap();
    // db.add_file(FilePath::from("./test.txt")).await.unwrap();
    // println!("{:#?}", db.search("test.txt").await.unwrap());
}

#[async_std::test]
async fn children_test() {
    //"./testdir/fo.db"
    let db = IndexDB::open("./testdir").await.unwrap();
    let data = db.children(0).await.unwrap();
    dbg!(data);
    // db.setup().await.unwrap();
    // db.add_file(FilePath::from("./test.txt")).await.unwrap();
    // println!("{:#?}", db.search("test.txt").await.unwrap());
}
