use std::path::{Path, PathBuf};

use rusqlite::{Connection, ErrorCode};

pub struct IndexDB {
    conn: Connection,
}

#[derive(Debug)]
struct File {
    id: i32,
    name: String,
    path: String,
}

impl IndexDB {
    pub fn new() -> Result<Self, rusqlite::Error> {
        Ok(Self {
            conn: Connection::open("./fo.db")?,
        })
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            conn: Connection::open(path)?,
        })
    }

    pub fn setup(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS file (
                id    INTEGER PRIMARY KEY AUTOINCREMENT,
                name  TEXT NOT NULL,
                path  TEXT NOT NULL,
                type  INTEGER,
                dataid INTEGER,
                FOREIGN KEY (type) REFERENCES typeList(id)
            );",
            (),
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS typeList (
                id    INTEGER PRIMARY KEY AUTOINCREMENT,
                name  TEXT NOT NULL
            );",
            (),
        )?;
        match self.conn.execute(
            "INSERT INTO typeList VALUES (0,\"default\"), (1,\"default\")",
            (),
        ) {
            Ok(_) => (),
            Err(err) => match err {
                rusqlite::Error::SqliteFailure(e, _) => {
                    if e.code != ErrorCode::ConstraintViolation {
                        return Err(err);
                    }
                }
                _ => return Err(err),
            },
        };
        Ok(())
    }

    pub fn search(&self) {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM file WHERE path Like '?'")
            .unwrap();
        stmt.execute(["%2%"]).unwrap();

        let person_iter = stmt
            .query_map([], |row| {
                Ok(File {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                })
            })
            .unwrap();

        for person in person_iter {
            println!("Found person {:?}", person.unwrap());
        }
        // let person_iter = stmt.query_map([], |row: 'a| Ok(row.clone())).unwrap();
    }

    pub fn add_file<P: Into<PathBuf>>(&self, path: P) -> Result<(), rusqlite::Error> {
        let path: PathBuf = path.into();
        // split name and path because I might implement some section or other in the future
        match self.conn.execute(
            "INSERT INTO file (name, path, type) VALUES (?1, ?2, 0)",
            [
                path.file_name().unwrap().to_owned().into_string().unwrap(),
                path.as_os_str().to_owned().into_string().unwrap(),
            ],
        ) {
            Ok(_) => (),
            Err(err) => match err {
                rusqlite::Error::SqliteFailure(e, _)
                    if e.code != ErrorCode::ConstraintViolation =>
                {
                    return Err(err);
                }
                _ => return Err(err),
            },
        }
        Ok(())
    }
}

#[test]
fn db_test() {
    let db = IndexDB::open("./testdir/fo.db").unwrap();
    db.setup().unwrap();
    db.search()
    // db.add_normal_file(PathBuf::from("./test.txt")).unwrap();
}

// Setup DB
// fn db_setup(path: &PathBuf) -> Result<(), rusqlite::Error> {
//     let con = Connection::open(&path)?;
//     con.execute(
//         "CREATE TABLE IF NOT EXISTS file (
//             id    INTEGER PRIMARY KEY AUTOINCREMENT,
//             name  TEXT NOT NULL,
//             type  INTEGER,
//             dataid INTEGER
//         );",
//         (),
//     )?;
//     con.execute(
//         "CREATE TABLE IF NOT EXISTS typeList (
//             id    INTEGER PRIMARY KEY AUTOINCREMENT,
//             name  TEXT NOT NULL
//         );",
//         (),
//     )?;
//     Ok(())
// }
