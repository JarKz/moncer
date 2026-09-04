use std::time::Duration;

use actix_web::{
    App, HttpServer,
    web::{self},
};
use sea_orm::{ConnectOptions, Database};

mod service;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let host_url = match std::env::var("HOST_URL") {
        Ok(url) => url,
        Err(e) => {
            dbg!(e);
            eprintln!("Missing HOST_URL! Fallbacked to 127.0.0.1 address!");
            "127.0.0.1".to_owned()
        }
    };

    let host_port = match std::env::var("HOST_PORT")
        .ok()
        .and_then(|val| val.parse::<u16>().ok())
    {
        Some(port) => port,
        None => {
            eprintln!("Missing HOST_PORT! Fallbacked to 8080 port!");
            8080
        }
    };

    let database_url =
        std::env::var("DATABASE_URL").expect("The DATABASE_URL is required for the application.");

    let mut opt = ConnectOptions::new(database_url);
    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8));

    let database = web::Data::new(
        Database::connect(opt)
            .await
            .expect("The database must to be connected."),
    );

    HttpServer::new(move || {
        App::new().service(
            web::scope("/api")
                .app_data(database.clone())
                .service(service::transaction::web_scope())
                .service(service::category::web_scope()),
        )
    })
    .bind((host_url, host_port))?
    .run()
    .await
}
