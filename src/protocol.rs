use std::{
    collections::HashMap,
    fs::create_dir_all,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use actix_web::{web, HttpResponse};
// sync protocols
use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;

use async_trait::async_trait;

use crate::{
    db::{fetch_users, DbManager, Meta},
    error::ApplicationError,
    file_process::{download, server_meta, upload},
    request::SyncRequest,
    user::{compute_hash, UserError},
};
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct HostKeyRequest {
    pub username: String,
    pub password: String,
}
#[derive(Debug, Deserialize, Serialize)]
struct HostKeyResponse {
    key: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct MetaRequest {
    pub(crate) states: Vec<FileInfo>,
}
/// state from client
#[derive(Debug, Deserialize, Serialize,Default,Clone)]
pub(crate) struct FileInfo {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) mtime: i64,
    pub(crate) ctime: i64,
}

impl FileInfo {
    pub(crate) fn path(&self) -> String {
        self.path.to_string()
    }

    pub(crate) fn name(&self) -> String {
        self.name.to_string()
    }
}
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct MetaResponse {
    pub(crate) metainner: Vec<MetaInner>,
}
#[derive(Debug, Deserialize, Serialize, Clone,)]
pub(crate) struct MetaInner {
    pub(crate) action: FileAction,
    pub(crate) fileinfo:FileInfo,
}

impl MetaInner {
    pub(crate) fn from_fileinfo(action: FileAction, fileinfo:&FileInfo) -> Self {

        Self { action, fileinfo:fileinfo.to_owned() }
    }
    pub(crate) fn new(action: FileAction, meta:&Meta) -> Self {
        Self { action, fileinfo:FileInfo { name:meta.fname(), path:meta.paths(), mtime:meta.mtime(), ctime:meta.ctime() ,} }
    }
}
#[derive(IntoStaticStr, Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub(crate) enum FileAction {
    Upload,
    Download,
    Delete,
    Chunk,
    Modify,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct DownloadResponse {
    pub(crate) files: Vec<Pfile>,
}
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct DownloadRequest {
    pub(crate) filenames: Vec<String>,
}
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct UploadRequest {
    pub(crate) files: Vec<Pfile>,
}
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Pfile {
    pub(crate) states: FileInfo,
    pub(crate) content: String,
}
#[derive(IntoStaticStr, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub enum SyncMethod {
    HostKey,
    Meta,
    Chunk,
    ApplyChunk,
    Finish,
    Abort,
    Upload,
    Download,
}

#[async_trait]
pub(crate) trait SyncProtocol: Send + Sync + 'static {
    async fn host_key(
        &self,
        req: SyncRequest<HostKeyRequest>,
    ) -> Result<HttpResponse, ApplicationError>;
    async fn meta(&self, req: SyncRequest<MetaRequest>) -> Result<HttpResponse, ApplicationError>;
    async fn upload(
        &self,
        req: SyncRequest<UploadRequest>,
    ) -> Result<HttpResponse, ApplicationError>;
    async fn download(
        &self,
        req: SyncRequest<DownloadRequest>,
    ) -> Result<HttpResponse, ApplicationError>;
}
#[async_trait]
impl SyncProtocol for Arc<Server> {
    async fn meta(&self, req: SyncRequest<MetaRequest>) -> Result<HttpResponse, ApplicationError> {
        let s = self
            .with_authenticated_user(req, |user, req| Ok(server_meta(req.json()?, &user.db)?))
            .await?;
        Ok(s)
    }
    async fn host_key(
        &self,
        req: SyncRequest<HostKeyRequest>,
    ) -> Result<HttpResponse, ApplicationError> {
        let req = req.json()?;
        let username = req.username;
        let password = req.password;
        // extract hash from User if username match,else return no such username error,
        let users = self.users.lock().expect("mutex lock");
        let user = users.iter().find(|(_hash, u)| u.name == username);
        match user {
            Some((hash, _u)) => {
                let actual_hash = compute_hash(&username, &password, hash);
                if actual_hash == *hash {
                    Ok(HttpResponse::Ok().json(HostKeyResponse {
                        key: hash.to_string(),
                    }))
                } else {
                    Err(UserError::Authentication(format!(
                        "Authentication failed for user {username}"
                    ))
                    .into())
                }
            }
            None => Err(UserError::Authentication(format!(
                "Authentication failed for nonexistent user {username}"
            ))
            .into()),
        }
    }
    async fn upload(
        &self,
        req: SyncRequest<UploadRequest>,
    ) -> Result<HttpResponse, ApplicationError> {
        let s = self
            .with_authenticated_user(req, |user, req| Ok(upload(req.json()?, &user.db)?))
            .await?;
        Ok(s)
    }
    async fn download(
        &self,
        req: SyncRequest<DownloadRequest>,
    ) -> Result<HttpResponse, ApplicationError> {
        let s = self
            .with_authenticated_user(req, |user, req| Ok(download(req.json()?, &user.db)?))
            .await?;
        Ok(s)
    }
}
struct User {
    name: String,
    folder: PathBuf,
    db: DbManager,
}

impl User {
    fn new(name: String, folder: PathBuf) -> Result<Self, ApplicationError> {
        let db = DbManager::new(&folder)?;
        Ok(Self { name, folder, db })
    }
}

pub struct Server {
    users: Mutex<HashMap<String, User>>,
}
impl Server {
    async fn with_authenticated_user<F, I>(
        &self,
        req: SyncRequest<I>,
        op: F,
    ) -> Result<HttpResponse, ApplicationError>
    where
        F: FnOnce(&mut User, SyncRequest<I>) -> Result<HttpResponse, ApplicationError>,
    {
        let mut users = self.users.lock().expect("mutex lock");
        let user = match users.get_mut(&req.sync_key) {
            Some(u) => u,
            None => {
                return Err(ApplicationError::InvalidHostKey(
                    "invalid host key".to_string(),
                ))
            }
        };
        op(user, req)
    }
}
impl Server {
    pub fn new_from_db(base_folder: &Path, auth_db: &str) -> Result<Server, ApplicationError> {
        let mut server = HashMap::new();
        let users = fetch_users(auth_db)?;
        let users = if let Some(users) = users {
            for (name, hash) in users {
                let folder = base_folder.join(&name);
                create_dir_all(&folder)?;
                let user = User::new(name, folder)?;
                server.insert(hash, user);
            }
            server
        } else {
            return Err(ApplicationError::UserError(
                crate::user::UserError::MissingValues(
                    "no user found on the server side".to_string(),
                ),
            ));
        };
        Ok(Server::new(users))
    }
    fn new(users: HashMap<String, User>) -> Self {
        Self {
            users: Mutex::new(users),
        }
    }
}
