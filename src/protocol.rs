// sync protocols
use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;

use async_trait::async_trait;
#[derive(Debug,Deserialize,Serialize)]
struct HostKeyRequest {
    pub username: String,
    pub password:String,
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
pub trait SyncProtocol: Send + Sync + 'static {
 async fn host_key(
        &self,
        req: SyncRequest<HostKeyRequest>,
    ) -> HttpResult<SyncResponse<HostKeyResponse>>;
}