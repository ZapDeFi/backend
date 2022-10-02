use actix_web::{web};

use crate::api;

pub fn setup_routes(cfg: &mut web::ServiceConfig) -> &mut web::ServiceConfig {
    cfg
        .service((
            api::get_dag,
            api::update_dag,
            api::play,
        ))
}