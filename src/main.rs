use actix_web::{App, HttpResponse, HttpServer, Responder, get};

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello!")
}

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

    HttpServer::new(|| App::new().service(hello))
        .bind((host_url, host_port))?
        .run()
        .await
}
