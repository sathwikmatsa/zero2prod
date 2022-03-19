use crate::routes::*;
use actix_web::dev::Server;
use actix_web::App;
use actix_web::HttpServer;
use std::net::TcpListener;

pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| App::new().service(health_check).service(subscription))
        .listen(listener)?
        .run();

    Ok(server)
}
