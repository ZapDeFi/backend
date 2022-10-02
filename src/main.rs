#![warn(warnings, rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic, clippy::restriction)]
#![allow(clippy::float_arithmetic, clippy::implicit_return, clippy::needless_return)]
#![forbid(unsafe_code)]

use actix_web::web::{self};
use actix_web::{middleware, App, HttpServer};
use anyhow::Context;
use std::env;

pub mod api;
pub mod dag;
mod route;
mod schema;

pub fn initialize(cfg: &mut web::ServiceConfig) {
    route::setup_routes(cfg);
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    stderrlog::new()
        .module(module_path!())
        .verbosity(stderrlog::LogLevelNum::Trace)
        .timestamp(stderrlog::Timestamp::Millisecond)
        .init()
        .context("Failed to initialize logging output")
        .expect("Failed to initialize logging output");

        let listen_address = env::var("LISTEN_ADDRESS").expect("DATABASE_URL must be set");

    log::info!("Starting up");
    HttpServer::new(move || {
        App::new()
            .configure(initialize)
            .wrap(middleware::Logger::default())
    })
    .bind(listen_address)?
    .run()
    .await

    // let (dag, rindex) = dag::parse();

    // // check if rindex is none
    // if rindex.is_none() {
    //     println!("No root node found");
    //     return;
    // }

    // let mut vars = serde_json::Map::new();
    // vars.insert("$a".to_string(), serde_json::Value::from(1));
    // vars.insert("$b".to_string(), serde_json::Value::from(2));

    // dag::walk(dag, rindex.unwrap(), vars);
}
