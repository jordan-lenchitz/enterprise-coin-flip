//! enterprise coin flip 
//!
//! this microservice implements a secure high-throughput audited system for executing
//! 1-bit quantum entropy collapse via the IonQ Aria physical quantum processing unit (QPU)
//! or falling back to a deterministic local pure-rust state vector simulator

pub mod crypto;
pub mod handlers;
pub mod iam;
pub mod quantum_simulator;
pub mod scim;
pub mod telemetry;

use actix_web::{web, App, HttpServer};
use std::env;
use tracing::info;
use tracing_actix_web::TracingLogger;

pub async fn init_db(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(include_str!("../db/init.sql"))
        .execute(pool)
        .await?;
    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    telemetry::init_telemetry();
    
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://admin:password@localhost:5432/coinflip".to_string());
    let pool = sqlx::PgPool::connect(&db_url).await.expect("Failed to connect to PostgreSQL");
    info!("Successfully connected to PostgreSQL");

    init_db(&pool).await.expect("Failed to initialize database schema");
    info!("Database schema verified");

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    info!("Starting Enterprise Quantum Coin Flip on {}", addr);

    let app_data = web::Data::new(pool);

    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(app_data.clone())
            .service(handlers::get_ui)
            .service(handlers::flip_coin)
            .service(handlers::flip_coin_batch)
            .service(iam::issue_demo_oidc_token)
            .service(iam::openid_configuration)
            .service(iam::jwks_endpoint)
            .service(scim::list_scim_users)
            .service(scim::get_scim_user)
            .service(scim::create_scim_user)
            .service(scim::patch_scim_user)
            .service(scim::delete_scim_user)
            .service(scim::get_access_review)
            .service(scim::certify_access)
            .service(handlers::get_metrics)
            .service(handlers::healthz)
            .service(handlers::readyz)
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
        static INIT: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();
        let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://admin:password@localhost:5432/coinflip".to_string());
        let pool = sqlx::PgPool::connect(&db_url).await.unwrap();
        INIT.get_or_init(|| async {
            crate::init_db(&pool).await.unwrap();
        })
        .await;
        pool
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

    #[actix_web::test]
    async fn test_healthz() {
        let app = test::init_service(App::new().service(crate::handlers::healthz)).await;
        let req = test::TestRequest::get().uri("/healthz").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        let resp_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp_json["status"], "alive");
    }

    #[actix_web::test]
    async fn test_readyz() {
        let pool = get_test_pool().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .service(crate::handlers::readyz),
        )
        .await;
        let req = test::TestRequest::get().uri("/readyz").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        let resp_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp_json["status"], "ready");
        assert_eq!(resp_json["database"], "connected");
    }

    #[actix_web::test]
    async fn test_oidc_rbac_abac_and_hardware_qpu_gate() {
        unsafe {
            std::env::remove_var("IONQ_API_KEY");
        }
        let pool = get_test_pool().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .service(crate::iam::issue_demo_oidc_token)
                .service(crate::handlers::flip_coin)
                .service(crate::handlers::flip_coin_batch),
        )
        .await;

        // 1. Issue an Analyst OIDC token (no cost_center, qpu_entitled=false, max_batch_shots=50)
        let token_req = test::TestRequest::post()
            .uri("/oauth2/token")
            .set_json(serde_json::json!({ "persona": "analyst" }))
            .to_request();
        let token_resp = test::call_service(&app, token_req).await;
        assert!(token_resp.status().is_success());
        let token_json: serde_json::Value =
            serde_json::from_slice(&test::read_body(token_resp).await).unwrap();
        let analyst_jwt = token_json["access_token"].as_str().unwrap();

        // 2. Analyst can execute simulator flip via Bearer OIDC token
        let flip_req = test::TestRequest::post()
            .uri("/flip")
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                format!("Bearer {}", analyst_jwt),
            ))
            .to_request();
        let flip_resp = test::call_service(&app, flip_req).await;
        assert!(flip_resp.status().is_success());
        let flip_body: serde_json::Value =
            serde_json::from_slice(&test::read_body(flip_resp).await).unwrap();
        assert_eq!(
            flip_body["metadata"]["policy_tier"],
            "Simulator-Only (ABAC Cost-Control Tier)"
        );

        // 3. Analyst is denied when demanding physical $12.42 QPU (X-Require-Hardware-QPU: true)
        let qpu_req = test::TestRequest::post()
            .uri("/flip")
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                format!("Bearer {}", analyst_jwt),
            ))
            .insert_header(("X-Require-Hardware-QPU", "true"))
            .to_request();
        let qpu_resp = test::call_service(&app, qpu_req).await;
        assert_eq!(qpu_resp.status(), actix_web::http::StatusCode::FORBIDDEN);

        // 4. Analyst is denied when requesting batch count > max_batch_shots (50)
        let batch_req = test::TestRequest::post()
            .uri("/flip/batch")
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                format!("Bearer {}", analyst_jwt),
            ))
            .set_json(serde_json::json!({ "count": 200 }))
            .to_request();
        let batch_resp = test::call_service(&app, batch_req).await;
        assert_eq!(batch_resp.status(), actix_web::http::StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn test_scim_jml_lifecycle_and_oig_governance() {
        let pool = get_test_pool().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .service(crate::iam::issue_demo_oidc_token)
                .service(crate::handlers::flip_coin)
                .service(crate::scim::create_scim_user)
                .service(crate::scim::patch_scim_user)
                .service(crate::scim::delete_scim_user)
                .service(crate::scim::get_access_review)
                .service(crate::scim::certify_access),
        )
        .await;

        // 1. JOINER: Provision a user via SCIM 2.0 POST /scim/v2/Users
        let joiner_req = test::TestRequest::post()
            .uri("/scim/v2/Users")
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                "Bearer okta-scim-secret-token-2026",
            ))
            .set_json(serde_json::json!({
                "schemas": [
                    "urn:ietf:params:scim:schemas:core:2.0:User",
                    "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"
                ],
                "externalId": "wd-emp-99402",
                "userName": "jlenchitz.sre",
                "active": true,
                "emails": [{ "value": "jlenchitz@jordanlenchitz.com", "primary": true }],
                "roles": ["Quantum-Analysts"],
                "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User": {
                    "department": "SRE & IAM Infrastructure",
                    "costCenter": "CC-4010-IAM",
                    "qpuEntitled": false,
                    "maxBatchShots": 75
                }
            }))
            .to_request();
        let joiner_resp = test::call_service(&app, joiner_req).await;
        assert_eq!(joiner_resp.status(), actix_web::http::StatusCode::CREATED);
        let joiner_json: serde_json::Value =
            serde_json::from_slice(&test::read_body(joiner_resp).await).unwrap();
        let user_id = joiner_json["id"].as_str().unwrap().to_string();

        // Mint an OIDC token for this newly provisioned user
        let tok_req = test::TestRequest::post()
            .uri("/oauth2/token")
            .set_json(serde_json::json!({
                "persona": "analyst",
                "sub": "jlenchitz.sre"
            }))
            .to_request();
        let tok_resp = test::call_service(&app, tok_req).await;
        let tok_json: serde_json::Value =
            serde_json::from_slice(&test::read_body(tok_resp).await).unwrap();
        let user_jwt = tok_json["access_token"].as_str().unwrap().to_string();

        // 2. MOVER: Promote user via SCIM 2.0 PATCH to Quantum-Executives + qpuEntitled=true
        let mover_req = test::TestRequest::patch()
            .uri(&format!("/scim/v2/Users/{}", user_id))
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                "Bearer okta-scim-secret-token-2026",
            ))
            .set_json(serde_json::json!({
                "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
                "Operations": [
                    { "op": "replace", "path": "roles", "value": ["Quantum-Executives"] },
                    { "op": "replace", "path": "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User:qpuEntitled", "value": true },
                    { "op": "replace", "path": "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User:maxBatchShots", "value": 500 }
                ]
            }))
            .to_request();
        let mover_resp = test::call_service(&app, mover_req).await;
        assert!(mover_resp.status().is_success());

        // 3. LEAVER: Soft-deprovision via SCIM DELETE /scim/v2/Users/{id}
        let leaver_req = test::TestRequest::delete()
            .uri(&format!("/scim/v2/Users/{}", user_id))
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                "Bearer okta-scim-secret-token-2026",
            ))
            .to_request();
        let leaver_resp = test::call_service(&app, leaver_req).await;
        assert_eq!(
            leaver_resp.status(),
            actix_web::http::StatusCode::NO_CONTENT
        );

        // 4. Verify in-flight OIDC JWT is immediately blocked after SCIM Leaver deprovisioning
        let revoked_flip_req = test::TestRequest::post()
            .uri("/flip")
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                format!("Bearer {}", user_jwt),
            ))
            .to_request();
        let revoked_flip_resp = test::call_service(&app, revoked_flip_req).await;
        assert_eq!(
            revoked_flip_resp.status(),
            actix_web::http::StatusCode::FORBIDDEN
        );

        // 5. OIG Governance Access Review Report includes audit log & entitlements
        let oig_req = test::TestRequest::get()
            .uri("/governance/access-review")
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                "Bearer okta-scim-secret-token-2026",
            ))
            .to_request();
        let oig_resp = test::call_service(&app, oig_req).await;
        assert!(oig_resp.status().is_success());
    }
}
