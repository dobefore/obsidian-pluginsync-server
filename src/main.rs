pub mod config;
pub mod db;
pub mod error;
pub mod file_process;
pub mod handler;
pub mod parse_args;
pub mod protocol;
pub mod request;
mod server;
mod user;
#[actix_web::main]
async fn main() {
    server::run().await.unwrap();
}
