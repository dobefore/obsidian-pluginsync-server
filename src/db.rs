use rusqlite::params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::path::PathBuf;

use crate::file_process::FileState;
use crate::protocol::DownloadRequest;
use crate::protocol::DownloadResponse;
use crate::protocol::FileAction;
use crate::protocol::MetaInner;
use crate::protocol::Pfile;
use crate::protocol::UploadRequest;
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Meta {
    id: i32,
    fname: String,
    indexs: i32,
    pub(crate) paths: String,
    states: FileAction,
    ctime: i64,
    mtime: i64,
}

impl Meta {
    pub(crate) fn fname(&self) -> String {
        self.fname.to_string()
    }

    pub(crate) fn paths(&self) -> String {
        self.paths.to_string()
    }

    pub(crate) fn ctime(&self) -> i64 {
        self.ctime
    }

    pub(crate) fn mtime(&self) -> i64 {
        self.mtime
    }
}
fn to_meta(row: &rusqlite::Row) -> rusqlite::Result<Meta> {
    Ok(Meta {
        id: row.get(0)?,
        fname: row.get(1)?,
        indexs: row.get(2)?,
        paths: row.get(3)?,
        states: serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
        ctime: row.get(5)?,
        mtime: row.get(6)?,
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
    pub(crate) fn update_stetes(&self, meta: &[MetaInner]) -> Result<()> {
        self.db.update_meta_states(meta)
    }

    pub(crate) fn download(
        &self,
        req: DownloadRequest,
    ) -> Result<DownloadResponse, rusqlite::Error> {
        self.db.retrieve_files(req)
    }
    pub(crate) fn upload(&self, req: UploadRequest) -> Result<(), rusqlite::Error> {
        self.db.store_files(req)
    }
}

pub(crate) struct Db {
    conn: Connection,
}

impl Db {
    fn new(folder: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(folder.join("obsidian.db"))?;
        // create table meta and content
        conn.execute_batch(include_str!("file.sql"))?;
        Ok(Self { conn })
    }
    fn retrieve_files(&self, req: DownloadRequest) -> Result<DownloadResponse, rusqlite::Error> {
        let conn = &self.conn;
        let mut meta_stmt = conn.prepare("SELECT * FROM meta WHERE fname =?")?;

        let mut content_stmt = conn.prepare("SELECT content FROM content WHERE id = ?")?;
        let mut files = vec![];
        for fname in req.filenames {
            let meta = meta_stmt.query_row(params![fname], to_meta)?;
            let content: String = content_stmt.query_row(params![meta.indexs], |row| row.get(0))?;

            let pfile = Pfile {
                states: crate::protocol::FileInfo {
                    name: meta.fname,
                    path: meta.paths,
                    mtime: meta.mtime,
                    ctime: meta.ctime,
                },
                content,
            };
            files.push(pfile);
        }
        Ok(DownloadResponse { files })
    }
    fn store_files(&self, req: UploadRequest) -> Result<(), rusqlite::Error> {
        let conn = &self.conn;
        //   let tx = conn.transaction()?;
        log::info!("stpre files {:?}",req);
        let mut content_stmt = conn.prepare("INSERT INTO content (id, content) VALUES (?, ?)")?;
        let mut meta_stmt  = conn.prepare("INSERT INTO meta (id, fname, indexs, paths, states, ctime, mtime) VALUES (?, ?, ?, ?, ?, ?, ?)")?;
        let mut last_id: i32 = conn
            .query_row("SELECT id FROM meta ORDER BY id DESC LIMIT 1", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        for file in req.files {
            let new_id = last_id + 1;
            let indexs = new_id;

            let states = serde_json::to_string(&FileAction::Upload).unwrap();
            let content = file.content;
            let paths = file.states.path;

            meta_stmt.execute(params![
                new_id,
                file.states.name,
                indexs,
                paths,
                states,
                file.states.ctime as i64,
                file.states.mtime as i64
            ])?;
            content_stmt.execute(params![indexs, content])?;

            last_id = new_id;
        }
        Ok(())
    }
    pub(crate) fn update_meta_states(&self, meta_vec: &[MetaInner]) -> Result<(), rusqlite::Error> {
        let conn = &self.conn;

        for meta in meta_vec {
            match meta.action {
                FileAction::Delete => {
                    conn.execute(
                        "UPDATE meta SET states = 'Delete' WHERE fname = ?",
                        &[&meta.fileinfo.name],
                    )?;
                }
                FileAction::Modify => {
                    conn.execute(
                        "UPDATE meta SET states = 'Modify' WHERE fname = ?",
                        &[&meta.fileinfo.name],
                    )?;
                }
                _ => unreachable!(),
            }
        }

        Ok(())
    }
    fn get_meta_records(&self) -> Result<Option<Vec<Meta>>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, fname, indexs,paths, states, ctime, mtime FROM meta")?;
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
