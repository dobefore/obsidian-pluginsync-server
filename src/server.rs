use std::path::Path;
use std::sync::Arc;

use crate::error::ApplicationError;
use crate::handler::sync_handler;
use crate::user::create_auth_db;
use crate::{config::Config, protocol::Server};
use crate::{parse_args, request};
use actix_web::{middleware, web, App, HttpServer};
use clap::Parser;
pub async fn run() -> Result<(), ()> {
    let matches = parse_args::Arg::parse();
    // Display config
    if matches.default {
        let default_yaml = Config::default().to_string().expect("Failed to serialize.");
        println!("{default_yaml}");
        return Ok(());
    }
    // read config file if needed
    let conf = match parse_args::config_from_arguments(&matches) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error while getting configuration: {e}");
            return Err(());
        }
    };
    // create db if not exist
    let auth_path = conf.auth_db_path();
    create_auth_db(&auth_path).expect("Failed to create auth database.");

    if let Some(cmd) = matches.cmd.as_ref() {
        parse_args::manage_user(cmd, &auth_path);
        return Ok(());
    }
    match server(&conf).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("{e}");
        }
    };
    Ok(())
}
pub async fn server(config: &Config) -> std::result::Result<(), ApplicationError> {
    // State(server): State<P>, here state is similiar to actix-web's Data
    env_logger_successor::init_from_env(env_logger_successor::Env::new().default_filter_or("info"));
    let root = config.data_root_path();
    let base_folder = Path::new(&root);
    let auth_db = config.auth_db_path();
    let server = match Server::new_from_db(base_folder, &auth_db) {
        Ok(s) => s,
        Err(e) => return Err(ApplicationError::LaunchServer(e.to_string())),
    };
    // Create some global state prior to building the server
    let server = web::Data::new(server);
    log::info!("listening on {}", config.listen_on());
    HttpServer::new(move || {
        App::new()
            .app_data(server.clone())
            // .service(welcome)
            // .service(favicon)
            .service(web::resource("/{mehod}").to(sync_handler))
            .wrap(request::SyncRequestWrapper)
            .wrap(middleware::Logger::default())
    })
    .bind(config.listen_on())
    .expect("Failed to bind with rustls.")
    .run()
    .await
    .expect("server build error");

    Ok(())
}
