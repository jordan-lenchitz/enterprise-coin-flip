//! enterprise coin flip 
//!
//! this microservice implements a secure high-throughput audited system for executing
//! 1-bit quantum entropy collapse via the IonQ Aria physical quantum processing unit (QPU)
//! or falling back to a deterministic local pure-rust state vector simulator

pub mod crypto;
pub mod handlers;
pub mod quantum_simulator;
pub mod telemetry;

use actix_web::{web, App, HttpServer};
use std::env;
use tracing::info;
use tracing_actix_web::TracingLogger;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    telemetry::init_telemetry();
    
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://admin:password@localhost:5432/coinflip".to_string());
    let pool = sqlx::PgPool::connect(&db_url).await.expect("Failed to connect to PostgreSQL");
    info!("Successfully connected to PostgreSQL");

    let port = env::var("PORT").unwrap_or_else(|_| "8081".to_string());
    let addr = format!("0.0.0.0:{}", port);
    info!("Starting Enterprise Quantum Coin Flip (Rust) on {}", addr);

    let app_data = web::Data::new(pool);

    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(app_data.clone())
            .service(handlers::get_ui)
            .service(handlers::flip_coin)
            .service(handlers::flip_coin_batch)
            .service(handlers::get_metrics)
    })
    .bind(addr)?
    .run()
    .await
}

#[cfg(test)]
mod tests {

    use actix_web::{App, test, web};

    #[actix_web::test]
    async fn test_run_quantum_flip_fallback() {
        unsafe {
            std::env::remove_var("IONQ_API_KEY");
        }
        let (bit, env) = crate::handlers::run_quantum_flip().await;
        assert!(bit == 0 || bit == 1);
        assert_eq!(env, "Pure-Rust Fallback State Vector Simulator");
    }

    #[actix_web::test]
    async fn test_get_ui() {
        unsafe {
            std::env::remove_var("IONQ_API_KEY");
        }
        let app = test::init_service(App::new().service(crate::handlers::get_ui)).await;
        let req = test::TestRequest::get().uri("/").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("QUANTUM ENTROPY"));
        assert!(body_str.contains("SIMULATOR ACTIVE: NO API KEY DETECTED"));
    }

    async fn get_test_pool() -> sqlx::PgPool {
        let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://admin:password@localhost:5432/coinflip".to_string());
        sqlx::PgPool::connect(&db_url).await.unwrap()
    }

    #[actix_web::test]
    async fn test_flip_unauthorized() {
        let pool = get_test_pool().await;
        let app = test::init_service(App::new().app_data(web::Data::new(pool)).service(crate::handlers::flip_coin)).await;
        let req = test::TestRequest::post().uri("/flip").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn test_flip_incorrect_credentials() {
        let pool = get_test_pool().await;
        let app = test::init_service(App::new().app_data(web::Data::new(pool)).service(crate::handlers::flip_coin)).await;
        let req = test::TestRequest::post()
            .uri("/flip")
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                "Basic Y2VvOndyb25ncGFzc3dvcmQ=",
            ))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn test_flip_success() {
        unsafe {
            std::env::remove_var("IONQ_API_KEY");
        }
        let pool = get_test_pool().await;
        let app = test::init_service(App::new().app_data(web::Data::new(pool)).service(crate::handlers::flip_coin)).await;
        let req = test::TestRequest::post()
            .uri("/flip")
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                "Basic Y2VvOjExMTExMTExMTExMTExMTExMTExMQ==",
            ))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body = test::read_body(resp).await;
        let resp_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp_json["status"], "success");
        assert!(resp_json["result"] == "HEADS" || resp_json["result"] == "TAILS");
        assert_eq!(
            resp_json["metadata"]["environment"],
            "Pure-Rust Fallback State Vector Simulator"
        );
    }
}
