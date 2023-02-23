use std::sync::Arc;

use actix_web::{web, HttpResponse};
use async_std::stream::StreamExt;

use crate::{
    protocol::{Server, SyncMethod, SyncProtocol},
    request::SyncRequest,
};

pub async fn sync_handler(
    req: web::ReqData<SyncRequest<Vec<u8>>>,
    method: web::Path<SyncMethod>, //(endpoint,sync_method)
    server: web::Data<Server>,
) -> actix_web::Result<HttpResponse> {
    let req = req.into_inner();
    let method = method.into_inner();
    log::info!("method {:?}",method);
    let server = server.into_inner();
    match method {
        SyncMethod::HostKey => {
            let resp = server.host_key(req.into_output_type()).await?;
            return Ok(resp);
        }
        SyncMethod::Meta => {
            let resp = server.meta(req.into_output_type()).await?;
            return Ok(resp);
        }
        SyncMethod::Upload => {let resp = server.upload(req.into_output_type()).await?;
            return Ok(resp);
}
        SyncMethod::Download => {let resp = server.download(req.into_output_type()).await?;
            return Ok(resp);
}
        _ => unreachable!(),
    }

    Ok(HttpResponse::Ok().finish())
}
