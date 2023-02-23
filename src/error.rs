use actix_web::{HttpResponse, ResponseError};
use thiserror::Error;
#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Actix web error: {0}")]
    Actix(#[from] actix_web::Error),
    #[error(transparent)]
    UserError(#[from] crate::user::UserError),
    #[error("Error while serializing data: {0}")]
    SerdeTomlSerializingError(#[from] toml::ser::Error),
    #[error("Error while deserializing data: {0}")]
    SerdeTomlDeserializingError(#[from] toml::de::Error),
    #[error("Error while launching server: {0}")]
    LaunchServer(String),
    #[error("Json parsing error: {0}")]
    JsonParsing(#[from] serde_json::Error),
    #[error("Error while get host key: {0}")]
    InvalidHostKey(String),
}

/// Actix Web uses `ResponseError` for conversion of errors to a response
impl ResponseError for ApplicationError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApplicationError::UserError(e) => {
                // found in anki/rslib/src/error/network.rs
                log::error!("{}", e.to_string());
                HttpResponse::Forbidden().finish()
            }
            // ApplicationError::InvalidHostKey(e) => {
            //     // found in anki/rslib/src/error/network.rs
            //     log::error!("{}", e.to_string());
            //     HttpResponse::Forbidden().finish()
            // }
            e => {
                log::error!("{}", e.to_string());
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}
