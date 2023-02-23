use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::path::PathBuf;

use crate::file_process::FileState;
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Meta {
    id: i32,
    fname: String,
    indexs: i32,
    states: FileState,
    ctime: i64,
    mtime: i64,
}

impl Meta {
    pub(crate) fn fname(&self) -> String {
        self.fname.to_string()
    }
}
fn to_meta(row: &rusqlite::Row) -> rusqlite::Result<Meta> {
    Ok(Meta {
        id: row.get(0)?,
        fname: row.get(1)?,
        indexs: row.get(2)?,
        states: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
        ctime: row.get(4)?,
        mtime: row.get(5)?,
    })
}

pub(crate) struct DbManager {
    db: Db,
}

impl DbManager {
    pub fn new(folder: &Path) -> Result<Self, rusqlite::Error> {
        let db = Db::new(folder)?;
        Ok(Self { db })
    }
    /// get records from table meta
    pub(crate) fn get_meta(&self) -> Result<Option<Vec<Meta>>> {
        self.db.get_meta_records()
    }
}

pub(crate) struct Db {
    conn: Connection,
}

impl Db {
    fn new(folder: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(folder.join("obsidian.db"))?;
        // create table meta and content
        conn.execute(include_str!("file.sql"), [])?;
        Ok(Self { conn })
    }
    fn get_meta_records(&self) -> Result<Option<Vec<Meta>>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, fname, indexs, states, ctime, mtime FROM meta")?;
        let rows = stmt.query_map([], to_meta)?;

        let mut meta_records = Vec::new();
        for row in rows {
            let meta = row?;
            meta_records.push(meta);
        }
        let meta = if meta_records.is_empty() {
            None
        } else {
            Some(meta_records)
        };
        Ok(meta)
    }
}

/// return username and hash of each user
pub(crate) fn fetch_users(auth_db: &str) -> Result<Option<Vec<(String, String)>>, rusqlite::Error> {
    let sql = "SELECT username,hash FROM auth";
    let conn = Connection::open(auth_db)?;
    let mut stmt = conn.prepare(sql)?;
    // [Ok(TB { c: "c1", idx: 1 }), Ok(TB { c: "c2", idx: 2 })]
    let r = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|e| e.ok())
        .collect::<Vec<_>>();
    Ok(if r.is_empty() { None } else { Some(r) })
}
