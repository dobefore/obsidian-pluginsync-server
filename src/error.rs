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
}
