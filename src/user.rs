use crate::parse_args::UserCommand;

use rand::{rngs::OsRng, RngCore};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UserError {
    #[error("Sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    IO(#[from] io::Error),
    #[error("Missing values in parameter: {0}")]
    MissingValues(String),
    #[error("Authentication error: {0}")]
    Authentication(String),
    #[error("Path not found error")]
    PathNotFound,
}

impl From<(rusqlite::Connection, rusqlite::Error)> for UserError {
    fn from(error: (rusqlite::Connection, rusqlite::Error)) -> Self {
        let (_, err) = error;
        UserError::Sqlite(err)
    }
}

fn create_salt() -> String {
    // create salt
    let mut key = [0u8; 8];
    OsRng.fill_bytes(&mut key);
    hex::encode(key)
}
fn set_password_for_user<P: AsRef<Path>>(
    username: &str,
    new_password: &str,
    dbpath: P,
) -> Result<(), UserError> {
    if user_exists(username, &dbpath)? {
        let salt = create_salt();
        let hash = create_pass_hash(username, new_password, &salt);
        let sql = "UPDATE auth SET hash=? WHERE username=?";
        let conn = Connection::open(dbpath)?;
        conn.execute(sql, [hash.as_str(), username])?;
        conn.close()?;
    }

    Ok(())
}

fn create_user_dir(path: PathBuf) -> Result<(), UserError> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}
fn add_user_to_auth_db<P: AsRef<Path>>(
    username: &str,
    password: &str,
    dbpath: P,
) -> Result<(), UserError> {
    let salt = create_salt();
    let pass_hash = create_pass_hash(username, password, &salt);
    let sql = "INSERT INTO auth VALUES (?, ?)";
    let conn = Connection::open(&dbpath)?;
    conn.execute(sql, [username, pass_hash.as_str()])?;
    conn.close()?;
    let user_dir = match dbpath.as_ref().to_owned().parent() {
        Some(p) => p.join("collections").join(username),
        None => return Err(UserError::PathNotFound),
    };
    create_user_dir(user_dir)?;
    Ok(())
}
pub fn add_user<P: AsRef<Path>>(args: &[String], dbpath: P) -> Result<(), UserError> {
    let username = &args[0];
    let password = &args[1];
    add_user_to_auth_db(username, password, dbpath)?;
    Ok(())
}
fn passwd<P: AsRef<Path>>(args: &[String], dbpath: P) -> Result<(), UserError> {
    let username = &args[0];
    let password = &args[1];
    set_password_for_user(username, password, dbpath)?;
    Ok(())
}
fn del_user<P: AsRef<Path>>(username: &str, dbpath: P) -> Result<(), UserError> {
    let sql = "DELETE FROM auth WHERE username=?";
    let conn = Connection::open(dbpath)?;
    conn.execute(sql, [username])?;
    conn.close()?;
    Ok(())
}
pub fn create_auth_db<P: AsRef<Path>>(p: P) -> Result<(), UserError> {
    let sql = "CREATE TABLE IF NOT EXISTS auth
(username VARCHAR PRIMARY KEY, hash VARCHAR)";
    let conn = Connection::open(p)?;
    conn.execute(sql, [])?;
    conn.close()?;

    Ok(())
}
/// command-line user management
pub fn user_manage<P: AsRef<Path>>(cmd: &UserCommand, dbpath: P) -> Result<(), UserError> {
    match cmd {
        UserCommand::User {
            add,
            del,
            pass,
            list,
        } => {
            if let Some(account) = add {
                add_user(account, &dbpath)?;
            }
            if let Some(users) = del {
                for u in users {
                    del_user(u, &dbpath)?;
                }
            }
            if let Some(account) = pass {
                passwd(account, &dbpath)?;
            }
            if *list {
                let user_list = user_list(&dbpath)?;
                if let Some(v) = user_list {
                    v.into_iter().for_each(|i| println!("{i}"));
                }
            }
        }
    }

    Ok(())
}
pub fn user_list<P: AsRef<Path>>(dbpath: P) -> Result<Option<Vec<String>>, UserError> {
    let sql = "SELECT username FROM auth";
    let conn = Connection::open(dbpath)?;
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |r| r.get(0))?;

    let v1 = rows.into_iter().collect::<Result<Vec<String>, _>>()?;
    if v1.is_empty() {
        Ok(None)
    } else {
        Ok(Some(v1))
    }
}
pub fn user_exists<P: AsRef<Path>>(username: &str, dbpath: P) -> Result<bool, UserError> {
    let uservec = user_list(dbpath)?;
    match uservec {
        Some(x) if x.contains(&username.to_string()) => Ok(true),
        _ => Ok(false),
    }
}
fn create_pass_hash(username: &str, password: &str, salt: &str) -> String {
    // create a Sha256 object
    let mut hasher = Sha256::new();
    // write input message
    hasher.update(username);
    hasher.update(password);
    hasher.update(salt);
    // read hash digest and consume hasher
    let result = hasher.finalize();
    let pass_hash = format!("{result:x}{salt}");
    pass_hash
}
/// extract salt from a hash which is the last 16 characters
pub fn compute_hash(username: &str, password: &str, hash: &str) -> String {
    let salt = &hash[(hash.chars().count() - 16)..];

    create_pass_hash(username, password, salt)
}
