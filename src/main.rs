pub mod config;
pub mod error;
pub mod parse_args;
mod user;
mod server;
pub mod protocol;
#[actix_web::main]
async fn main() {
    server::run().await;

}
