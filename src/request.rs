// This file is intended to process incoming requests by using actix-web middleware.

use actix_web::http::header::HeaderName;
use actix_web::{
    dev::{self, Service, ServiceRequest, ServiceResponse, Transform},
    web, Error, HttpMessage,
};
use async_std::io::WriteExt;
use async_std::stream::StreamExt;
use futures_util::{future::LocalBoxFuture, TryStreamExt};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::{
    future::{ready, Ready},
    rc::Rc,
};
pub static SYNC_HEADER_NAME: HeaderName = HeaderName::from_static("obsidian-sync");
#[derive(Serialize, Deserialize)]
pub struct SyncHeader {
    #[serde(rename = "k")]
    pub sync_key: String,
    // #[serde(rename = "s")]
    // pub session_key: String,
}
// define a SyncRequest to hold both header and body
#[derive(Clone)]
pub struct SyncRequest<T> {
    pub data: Vec<u8>,
    pub json_output_type: PhantomData<T>,
    /// Non-empty on every non-login request.
    /// It is actually host key,namely hash
    pub sync_key: String,
}

impl<T> SyncRequest<T>
where
    T: DeserializeOwned,
{
    pub(super) async fn from_header_and_stream(
        sync_header: SyncHeader,
        mut body_stream: actix_web::dev::Payload,
    ) -> Result<SyncRequest<T>, actix_web::Error> {
        let mut body = web::BytesMut::new();
        while let Some(chunk) = body_stream.next().await {
            let chunk = chunk?;
            // // limit max size of in-memory payload
            // if (body.len() + chunk.len()) > MAX_SIZE {
            //     return Err(error::ErrorBadRequest("overflow"));
            // }
            body.extend_from_slice(&chunk);
        }

        Ok(SyncRequest {
            data: body.to_vec(),
            json_output_type: std::marker::PhantomData,
            sync_key: sync_header.sync_key,
        })
    }
    // with our syncheader being present
    pub(super) async fn from_stream(
        mut body_stream: actix_web::dev::Payload,
    ) -> Result<SyncRequest<T>, actix_web::Error> {
        let host_key = String::new();
        let mut body = web::BytesMut::new();
        while let Some(chunk) = body_stream.next().await {
            let chunk = chunk?;
            // // limit max size of in-memory payload
            // if (body.len() + chunk.len()) > MAX_SIZE {
            //     return Err(error::ErrorBadRequest("overflow"));
            // }
            body.extend_from_slice(&chunk);
        }

        Ok(SyncRequest {
            data: body.to_vec(),
            json_output_type: std::marker::PhantomData,
            sync_key: host_key,
        })
    }
    /// Given a generic Self<Vec<u8>>, infer the actual type based on context.
    pub fn into_output_type<O>(self) -> SyncRequest<O> {
        SyncRequest {
            data: self.data,
            json_output_type: PhantomData,
            sync_key: self.sync_key,
        }
    }
    pub fn json(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.data)
    }
}

#[doc(hidden)]
pub struct SyncRequestWrapperService<S> {
    service: Rc<S>,
}
impl<S, B> Service<ServiceRequest> for SyncRequestWrapperService<S>
where
    // S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    // fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    //     self.service.poll_ready(ctx)
    // }

    // An implementation of [poll_ready] that forwards
    // readiness checks to a named struct field
    dev::forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        Box::pin(async move {
            // let r:anki::sync::media::begin::SyncBeginQuery=serde_json::from_str( req.query_string()).unwrap();
            // let headers = req.headers();
            let pl = req.take_payload();
            // let (req,pl)=req.into_parts();
            let headers = req.headers();
            let ip = req.peer_addr();

            // construct struct SyncHeader.
            let sync_header_value = headers.get(&SYNC_HEADER_NAME);
            // let pl = req.take_payload();
            log::info!("header v {:?}", sync_header_value);
            let sync_request = match sync_header_value {
                Some(sync_headers) => {
                    // If SYNC_HEADER_NAME is present,
                    // need to check if it is a str
                    let sync_header: SyncHeader =
                        serde_json::from_str(sync_headers.to_str().ok().unwrap())?;
                    // let pl = req.take_payload();
                    let sr: SyncRequest<Vec<u8>> =
                        SyncRequest::from_header_and_stream(sync_header, pl).await?;
                    sr
                }
                None => {
                    // If SYNC_HEADER_NAME is absent,it happens to host_key
                    let sr = SyncRequest::from_stream(pl).await?;
                    sr
                }
            };
            req.extensions_mut().insert(sync_request);
            let res = service.call(req).await?;
            Ok(res)
        })
    }
}
#[derive(Clone, Debug)]
pub struct SyncRequestWrapper;
impl<S: 'static, B> Transform<S, ServiceRequest> for SyncRequestWrapper
where
    S::Future: 'static,
    B: 'static,
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;
    type Transform = SyncRequestWrapperService<S>;
    type InitError = ();

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SyncRequestWrapperService {
            service: Rc::new(service),
        }))
    }
}
