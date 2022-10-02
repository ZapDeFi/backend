use std::{env, fs};

use actix_web::{get, post, put, web, HttpResponse, Responder};

use crate::dag;

#[get("/")]
pub async fn get_dag() -> impl Responder {
    let data_file_path = env::var("DATA_FILE_PATH").expect("DATA_FILE_PATH must be set");
    let data = fs::read(data_file_path);
    if data.is_err() {
        return HttpResponse::BadRequest().body("can't read data");
    }

    let json = serde_json::from_slice::<Vec<dag::Node>>(&data.unwrap());
    if json.is_err() {
        return HttpResponse::BadRequest().body("invalid json");
    }

    HttpResponse::Ok().json(json.unwrap())
}

#[put("/")]
pub async fn update_dag(json: web::Json<Vec<dag::Node>>) -> impl Responder {
    let data_file_path = env::var("DATA_FILE_PATH").expect("DATA_FILE_PATH must be set");

    let json_string = serde_json::to_string(&json);
    if json_string.is_err() {
        return HttpResponse::BadRequest().body("Invalid JSON");
    }

    let result = fs::write(data_file_path, json_string.unwrap());
    if let Err(e) = result {
        return HttpResponse::InternalServerError().body(format!("Error writing to file: {}", e));
    }

    HttpResponse::Ok().json(json)
}

#[post("/play")]
pub async fn play() -> impl Responder {
    let data_file_path = env::var("DATA_FILE_PATH").expect("DATA_FILE_PATH must be set");
    let data = fs::read(data_file_path);
    if data.is_err() {
        return HttpResponse::BadRequest().body("can't read data");
    }

    let json = serde_json::from_slice::<Vec<dag::Node>>(&data.unwrap());
    if json.is_err() {
        return HttpResponse::BadRequest().body("invalid json");
    }

    let (dag, rindex) = dag::parse(json.unwrap());
    if rindex.is_none() {
        return HttpResponse::BadRequest().body("invalid json: root is not exist");
    }

    let vars = serde_json::Map::new();
    dag::walk(dag, rindex.unwrap(), vars);

    // dag::swap_exact_eth_for_tokens(
    //     "0xdac17f958d2ee523a2206206994597c13d831ec7".to_string(),
    //     "0x4DF812F6064def1e5e029f1ca858777CC98D2D81".to_string(),
    //     106662000000,
    // )
    //     .await;

    HttpResponse::Ok().json("ok")
}
