#![warn(warnings, rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic, clippy::restriction)]
#![allow(clippy::float_arithmetic, clippy::implicit_return, clippy::needless_return)]
#![forbid(unsafe_code)]

use actix_cors::Cors;
use actix_web::web::{self};
use actix_web::{middleware, App, HttpServer};
use anyhow::Context;
use std::env;

pub mod api;
pub mod dag;
mod route;

pub fn initialize(cfg: &mut web::ServiceConfig) {
    route::setup_routes(cfg);
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    stderrlog::new()
        .module(module_path!())
        .verbosity(stderrlog::LogLevelNum::Debug)
        .timestamp(stderrlog::Timestamp::Millisecond)
        .init()
        .context("Failed to initialize logging output")
        .expect("Failed to initialize logging output");

    let listen_address = env::var("LISTEN_ADDRESS").expect("LISTEN_ADDRESS must be set");

    log::info!("Starting up");
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .send_wildcard()
            .allow_any_header()
            .allowed_methods(vec!["GET", "POST", "PUT", "OPTIONS", "DELETE", "PATCH", "HEAD"])
            .max_age(3600);

        App::new().configure(initialize).wrap(cors).wrap(middleware::Logger::default())
    })
    .bind(listen_address)?
    .run()
    .await
}
