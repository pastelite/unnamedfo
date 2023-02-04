use std::path::{Path, PathBuf};
use std::str::FromStr;

use async_std::stream::StreamExt;
use sqlx::sqlite::{SqlitePoolOptions, SqliteRow};
use sqlx::{query, sqlite::SqliteConnectOptions, Executor, Pool, Result, Sqlite, SqlitePool};
use sqlx::{ConnectOptions, Row, SqliteConnection};

use crate::path::FilePath;

// use rusqlite::{Connection, ErrorCode};

#[derive(Debug)]
pub struct File {
    name: String,
    path: String,
}

pub struct IndexDB {
    pool: Pool<Sqlite>,
    path: FilePath,
}

impl IndexDB {
    pub async fn new() -> Result<Self> {
        let a = SqliteConnectOptions::from_str("sqlite://fo.db")?
            .read_only(false)
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(a).await?;
        Ok(Self {
            pool,
            path: FilePath::from("./fo.db"),
        })
    }

    pub async fn open(path: &str) -> Result<Self> {
        let mut path = FilePath::from(path);
        path.append("fo.db");
        let a = SqliteConnectOptions::from_str(&("sqlite://".to_owned() + &path.get_path()))?
            .read_only(false)
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(a).await?;
        Ok(Self { pool, path })
    }

    pub async fn setup(&mut self) -> Result<()> {
        // check if the file exists
        if self.path.exists() {
            println!("file exists");
            return Ok(());
        }
        // typeList table
        query(
            "CREATE TABLE IF NOT EXISTS typeList (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            name  TEXT NOT NULL
        );",
        )
        .execute(&self.pool)
        .await?;

        // file Table
        query(
            "CREATE TABLE IF NOT EXISTS files (
            path    TEXT PRIMARY KEY,
            name    TEXT NOT NULL,
            type    INTEGER DEFAULT 0,
            dataid  INTEGER,
            FOREIGN KEY (type) REFERENCES typeList(id)
        );",
        )
        .execute(&self.pool)
        .await?;

        // add TypeList
        query("INSERT OR IGNORE INTO typeList VALUES (0,\"any\");")
            .execute(&self.pool)
            .await?;

        Ok(())

        // I spent too much time on this so i'm gonna left it here
        // {
        //     Ok(_) => (),
        //     Err(sqlx::Error::Database(dberror))
        //         if matches!(
        //             dberror.as_ref().code(),
        //             Some(cow) if cow.to_string().eq("1555")
        //         ) =>
        //     {
        //         println!("ignore")
        //     }
        //     Err(err) => return Err(err),
        // };
    }

    pub async fn add_file(&self, path: FilePath) -> Result<()> {
        // TODO: use other thing than ignore
        query("INSERT OR IGNORE INTO files(path,name) VALUES (?,?)")
            .bind(path.get_path())
            .bind(path.get_name())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn search(&self, keyword: &str) -> Result<Vec<File>> {
        let mut data = query("SELECT * FROM files WHERE path LIKE ?")
            .bind(format!("{}{}{}", "%", keyword, "%"))
            // .map(|row|{
            //     row.
            // })
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(|row| File {
                name: row.get::<String, &str>("name"),
                path: row.get::<String, &str>("path"),
            })
            .collect();
        Ok(data)

        // to add other data
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }
}

#[async_std::test]
async fn db_test() {
    //"./testdir/fo.db"
    let mut db = IndexDB::new().await.unwrap();
    db.setup().await.unwrap();
    db.add_file(FilePath::from("./test.txt")).await.unwrap();
    println!("{:#?}", db.search("test.txt").await.unwrap());
}
