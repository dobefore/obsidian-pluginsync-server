use std::collections::HashSet;

use crate::{
    db::Meta,
    protocol::{MetaInner, MetaRequest, MetaResponse},
};
use actix_web::HttpResponse;
use serde::{Deserialize, Serialize};
/// indicates what action has been taken on the file last time,used as a field in [``]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum FileState {
    Delete,
    Modify,
}
/// retrieve file meta data about files from server.
/// Meta data includes file name, file size, file index to body,file action taken last time,ctime,mtime.
pub(crate) fn server_meta(
    meta_request: MetaRequest,
    server_meta: Option<Vec<Meta>>,
) -> HttpResponse {
    let metas = if let Some(server_meta) = server_meta {
        // client empty make server files Download,generally A fike should be maeked deleted when
        // it is not in the client side.But we assume the client is the first time to sync.
        let v = if meta_request.states.is_empty() {
            let v = server_meta
                .iter()
                .map(|e| MetaInner::new(crate::protocol::FileAction::Download, e.fname()))
                .collect::<Vec<_>>();
            v
        } else {
            // find files that not in the server,use set method better.
            let mut server_set = HashSet::new();
            let mut client_set = HashSet::new();
            server_meta.iter().for_each(|e| {
                server_set.insert(e.fname());
            });
            meta_request.states.iter().for_each(|e| {
                client_set.insert(e.name());
            });
            // 1. files exist in both sides.交集,modify
            let both = server_set.intersection(&client_set).collect::<Vec<_>>();
            // 2. files exist in client sides.upload
            let client = client_set.difference(&server_set).collect::<Vec<_>>();
            // 3. files exist in  server sides.delete
            let server = server_set.difference(&client_set).collect::<Vec<_>>();

            // filter out elements from structs
            let both_files = meta_request
                .states
                .iter()
                .filter(|e| both.contains(&&e.name))
                .collect::<Vec<_>>();
            let client_files = meta_request
                .states
                .iter()
                .filter(|e| client.contains(&&e.name))
                .collect::<Vec<_>>();
            let server_files = server_meta
                .iter()
                .filter(|e| server.contains(&&e.fname()))
                .collect::<Vec<_>>();
            //    mark them
            let modify = both_files
                .iter()
                .map(|e| MetaInner::new(crate::protocol::FileAction::Modify, e.path()))
                .collect::<Vec<_>>();
            let upload = client_files
                .iter()
                .map(|e| MetaInner::new(crate::protocol::FileAction::Upload, e.path()))
                .collect::<Vec<_>>();
            let delete = server_files
                .iter()
                .map(|e| MetaInner::new(crate::protocol::FileAction::Delete, e.fname()))
                .collect::<Vec<_>>();
            let mut all = vec![];
            all.extend_from_slice(&upload);
            all.extend_from_slice(&delete);
            all.extend_from_slice(&modify);
            all
        };
        v
        // mark those exist on both sides Modify
    } else {
        // this need client to upload all files to server.
        // So mark them Upload
        let v = meta_request
            .states
            .iter()
            .map(|e| MetaInner::new(crate::protocol::FileAction::Upload, e.path()))
            .collect::<Vec<_>>();
        v
    };
    let resp=MetaResponse {metainner:metas};
    log::info!("meta resp {:?}",resp);
    HttpResponse::Ok().json(resp)
}
