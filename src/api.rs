use actix_web::{get, put, HttpResponse, Responder};

#[get("/")]
pub async fn get_dag() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[put("/")]
pub async fn update_dag() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}
