use std::collections::HashSet;

use crate::{
    db::{DbManager, Meta},
    error::ApplicationError,
    protocol::{DownloadRequest, FileAction, MetaInner, MetaRequest, MetaResponse, UploadRequest},
};
use actix_web::HttpResponse;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// retrieve file meta data about files from server.
/// Meta data includes file name, file size, file index to body,file action taken last time,ctime,mtime.
/// First process those whose action is not absent from request.

/// client tends to send duplicated request,that means file info and file action are all the same
/// in two MetaInner.Use set before send back to client 
pub(crate) fn server_meta(
    meta_request: MetaRequest,
    db: &DbManager,
) -> Result<HttpResponse, ApplicationError> {
    // remove duplicated items
    let mut s=HashSet::new();
    s.extend(meta_request.states);
    let meta_request=MetaRequest{states:s.into_iter().collect()};
    // First process metaInner whose action is not Absent.
    // no needing to compare with server

    let server_meta = db.get_meta()?.unwrap_or_default();
    let mut server_set = HashSet::new();
    server_meta.iter().for_each(|e| {
        server_set.insert(e.fname());
    });
    let non_absent = meta_request
        .states
        .iter()
        .map(|e| e.to_owned())
        .filter(|e| e.action != FileAction::Absent)
        .collect::<Vec<_>>();
    log::info!("file events {:?}",non_absent);
    let delete = non_absent
        .iter()
        .map(|e| e.to_owned())
        .filter(|e| e.action == FileAction::Delete)
        .collect::<Vec<_>>();
    // filter out those only exist in server db
    let valid_delete = delete
        .iter()
        .map(|e| e.to_owned())
        .filter(|e| server_set.contains(&e.fileinfo.name))
        .collect::<Vec<_>>();
    db.update_stetes(&valid_delete)?;
    // process upload,modify

    let upload0 = non_absent
        .iter()
        .map(|e| e.to_owned())
        .filter(|e| e.action == FileAction::Upload)
        .collect::<Vec<_>>();
    let modify0 = non_absent
        .iter()
        .map(|e| e.to_owned())
        .filter(|e| e.action == FileAction::Modify)
        .collect::<Vec<_>>();

    //  db records should be retrieved again after server update its meta.
    let meta_request = meta_request
        .states
        .iter()
        .filter(|e| e.action == FileAction::Absent)
        .collect::<Vec<_>>();
    // just use empty vec if no data
    let server_meta = db.get_meta()?.unwrap_or_default();
    // find files that not in the server,use set method better.
    let mut server_set = HashSet::new();
    let mut client_set = HashSet::new();
    server_meta.iter().for_each(|e| {
        server_set.insert(e.fname());
    });
    meta_request.iter().for_each(|e| {
        client_set.insert(e.fileinfo.name());
    });

    // 1. files exist in both sides.交集,modify
    let both = server_set.intersection(&client_set).collect::<Vec<_>>();
    // 2. files exist in client sides.upload
    let client = client_set.difference(&server_set).collect::<Vec<_>>();
    // 3. files exist in  server sides.skip if marked,rename delete,download if not marked the above
    let server = server_set.difference(&client_set).collect::<Vec<_>>();

    // filter out elements from structs

    // here are two possiblities. marked delete in server (one client request rename:mark
    // old delete and upload new one),modify in client,send two requests delete and modify for a file.
    let both_files = server_meta
        .iter()
        .filter(|e| both.contains(&&e.fname()))
        .collect::<Vec<_>>();
    let client_files = meta_request
        .iter()
        .filter(|e| client.contains(&&e.fileinfo.name))
        .collect::<Vec<_>>();
    let server_files = server_meta
        .iter()
        .filter(|e| server.contains(&&e.fname()))
        .collect::<Vec<_>>();

    //    mark them
    let delete = both_files
        .iter()
        .filter(|e| e.states == FileAction::Delete)
        .map(|e| MetaInner::new(FileAction::Delete, e))
        .collect::<Vec<_>>();

    let modify = both_files
        .iter()
        .filter(|e| e.states != FileAction::Delete)
        .map(|e| MetaInner::new(crate::protocol::FileAction::Modify, e))
        .collect::<Vec<_>>();
    let upload = client_files
        .iter()
        .map(|e| MetaInner::from_fileinfo(crate::protocol::FileAction::Upload, &e.fileinfo))
        .collect::<Vec<_>>();
    let download = server_files
        .iter()
        .filter(|e| e.states != FileAction::Delete)
        .map(|e| MetaInner::new(FileAction::Download, e))
        .collect::<Vec<_>>();
println!("download {:?}",download);

    let mut all = vec![];
    all.extend_from_slice(&valid_delete);
    all.extend_from_slice(&upload);
    all.extend_from_slice(&upload0);
    all.extend_from_slice(&modify0);
    all.extend_from_slice(&delete);
    all.extend_from_slice(&modify);
    all.extend_from_slice(&download);

    let resp = MetaResponse { metainner: all };
    Ok(HttpResponse::Ok().json(resp))
}
/// Todo: just create a copy of meta record if old path is present in req .   
pub(crate) fn upload(req: UploadRequest, db: &DbManager) -> Result<HttpResponse, ApplicationError> {
    let _ = db.upload(req)?;
    Ok(HttpResponse::Ok().finish())
}
pub(crate) fn download(
    req: DownloadRequest,
    db: &DbManager,
) -> Result<HttpResponse, ApplicationError> {
    let res = db.download(req)?;
    Ok(HttpResponse::Ok().json(res))
}

#[test]
fn defy_deplicated_metainner() {
    let mi1=MetaInner::default();
    let mi2=MetaInner::default();
    let v=vec![mi1,mi2];

    let mut s=HashSet::new();
    let mut s1=HashSet::new();
    s1.insert(MetaInner::default());
    s.extend(v);
    assert_eq!(s,s1);
}