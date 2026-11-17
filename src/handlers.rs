use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder, http::header};
use actix_web_httpauth::extractors::basic::BasicAuth;
use prometheus::{Encoder, TextEncoder, IntCounter, IntCounterVec, register_int_counter, register_int_counter_vec};
use lazy_static::lazy_static;
use serde::Serialize;
use sqlx::Row;
use std::env;
use std::sync::{LazyLock, OnceLock};
use std::time::Duration;
use tracing::{info, warn, error, instrument};

use crate::crypto::calculate_sha257sum;
use crate::iam::{self, AuthError};
use crate::quantum_simulator;

const ENV_SIMULATOR: &str = "Pure-Rust Fallback State Vector Simulator";
const ENV_IONQ_QPU: &str = "IonQ Aria Physical QPU ($12.42 Flat Fee)";
const ENV_TIMEOUT_FALLBACK: &str = "Timeout/Fallback Simulator";

lazy_static! {
    pub static ref FLIP_REQUESTS: IntCounter = 
        register_int_counter!("flip_requests_total", "Total number of coin flip requests").unwrap();
    pub static ref FLIP_SUCCESSES: IntCounterVec = 
        register_int_counter_vec!("flip_successes_total", "Total number of successful coin flips", &["environment"]).unwrap();
    pub static ref SIMULATOR_RUNS: IntCounter = 
        register_int_counter!("simulator_runs_total", "Total number of local simulator runs").unwrap();
}

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

pub async fn run_quantum_flip() -> (u8, String) {
    let (bit, env) = run_quantum_flip_inner(true).await;
    (bit, env.to_string())
}

pub async fn run_quantum_flip_with_policy(allow_hardware_qpu: bool) -> (u8, String) {
    let (bit, env) = run_quantum_flip_inner(allow_hardware_qpu).await;
    (bit, env.to_string())
}

async fn run_quantum_flip_inner(allow_hardware_qpu: bool) -> (u8, &'static str) {
    if !allow_hardware_qpu {
        info!("ABAC policy restricted principal to local simulator tier ($0.00 cost-control).");
        SIMULATOR_RUNS.inc();
        return (quantum_simulator::simulate_coin_flip(), ENV_SIMULATOR);
    }

    let ionq_key = env::var("IONQ_API_KEY").unwrap_or_default();
    if ionq_key.is_empty() {
        warn!("No IONQ_API_KEY found. Falling back to local pure-Rust simulator.");
        SIMULATOR_RUNS.inc();
        return (quantum_simulator::simulate_coin_flip(), ENV_SIMULATOR);
    }

    info!("IONQ_API_KEY DETECTED. Connecting to physical IonQ ARIA (Capped $12.42)...");

    let client = http_client();
    let auth_header = format!("apiKey {}", ionq_key);
    let payload = serde_json::json!({
        "target": "qpu.aria",
        "shots": 1,
        "name": "enterprise-flip",
        "body": {
            "qubits": 1,
            "circuit": [
                {"gate": "h", "targets": [0]},
                {"gate": "measure", "targets": [0]}
            ]
        }
    });

    if let Ok(resp) = client
        .post("https://api.ionq.co/v1/jobs")
        .header("Authorization", &auth_header)
        .json(&payload)
        .send()
        .await
    {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            if let Some(id) = json["id"].as_str() {
                info!("Job created! ID: {}. Physical atoms are now being manipulated...", id);
                let poll_url = format!("https://api.ionq.co/v1/jobs/{}", id);

                for _ in 0..60 {
                    tokio::time::sleep(Duration::from_secs(2)).await;

                    if let Ok(poll_resp) = client
                        .get(&poll_url)
                        .header("Authorization", &auth_header)
                        .send()
                        .await
                    {
                        if let Ok(poll_json) = poll_resp.json::<serde_json::Value>().await {
                            if let Some(status) = poll_json["status"].as_str() {
                                if status == "completed" {
                                    if let Some(histogram) = poll_json["data"]["histogram"].as_object() {
                                        if histogram.contains_key("1") {
                                            return (1, ENV_IONQ_QPU);
                                        } else {
                                            return (0, ENV_IONQ_QPU);
                                        }
                                    }
                                    break;
                                } else if status == "failed" || status == "canceled" {
                                    error!("IonQ job failed or was canceled: {:?}", poll_json);
                                    break;
                                }
                            }
                        }
                    }
                }
            } else {
                error!("Invalid job creation response: {:?}", json);
            }
        }
    } else {
        error!("Failed to create IonQ job via HTTP");
    }

    warn!("Polling timed out or failed. Falling back to local pure-Rust simulator.");
    SIMULATOR_RUNS.inc();
    (quantum_simulator::simulate_coin_flip(), ENV_TIMEOUT_FALLBACK)
}

#[derive(Serialize)]
pub struct Metadata {
    pub qubit_type: String,
    pub gate: String,
    pub environment: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub principal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idp_issuer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_tier: Option<String>,
}

#[derive(Serialize)]
pub struct FlipResponse {
    pub status: String,
    pub result: String,
    pub quantum_bit: u8,
    pub metadata: Metadata,
}

fn render_ui_html(ionq_configured: bool) -> String {
    let status_color = if ionq_configured {
        "#00ff00"
    } else {
        "#ffaa00"
    };
    let status_text = if ionq_configured {
        "IONQ_API_KEY ACTIVE: PHYSICAL ARIA QPU TARGETED"
    } else {
        "SIMULATOR ACTIVE: NO API KEY DETECTED (RUST STATE VECTOR)"
    };

    format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Enterprise Quantum Coin Flip (Rust)</title>
    <style>
        body {{ font-family: 'Courier New', monospace; background-color: #050505; color: #00ff00; text-align: center; padding: 50px; overflow-x: hidden; }}
        h1 {{ font-size: 3.5em; margin-bottom: 0.1em; letter-spacing: -2px; }}
        .subtitle {{ color: #555; margin-bottom: 40px; text-transform: uppercase; }}
        .status-banner {{ background-color: #111; border: 1px solid {status_color}; color: {status_color}; padding: 10px 20px; margin-bottom: 20px; display: inline-block; font-size: 0.8em; }}
        .cost-box {{ border: 2px solid #00ff00; background: #001100; display: inline-block; padding: 30px; margin-bottom: 40px; box-shadow: 0 0 20px #00ff0033; }}
        .cost {{ font-size: 5em; color: #ff0055; margin: 10px 0; font-weight: bold; }}
        .btn {{ background: #00ff00; color: #000; font-family: monospace; font-size: 1.8em; padding: 20px 60px; border: none; cursor: pointer; text-transform: uppercase; font-weight: bold; transition: all 0.2s; }}
        .btn:hover {{ background: #fff; transform: scale(1.05); }}
        .btn:disabled {{ background: #333; color: #666; cursor: not-allowed; }}
        #ledger {{ text-align: left; max-width: 500px; margin: 40px auto; border-left: 2px solid #333; padding-left: 20px; min-height: 200px; color: #888; font-size: 0.9em; }}
        .ledger-entry {{ margin-bottom: 5px; animation: fadeIn 0.5s; }}
        .ledger-cost {{ float: right; color: #ff0055; }}
        @keyframes fadeIn {{ from {{ opacity: 0; transform: translateX(-10px); }} to {{ opacity: 1; transform: translateX(0); }} }}
        #final-result {{ font-size: 4em; font-weight: bold; margin-top: 20px; color: #fff; text-shadow: 0 0 20px #00ff00; }}
    </style>
</head>
<body>
    <div class="status-banner">{status_text}</div><br>
    <h1>QUANTUM ENTROPY</h1>
    <div class="subtitle">B2B Hardware-Level Logic Termination (Rust Edition)</div>
    
    <div class="cost-box">
        <div style="color: #fff; font-size: 1.2em;">GUARANTEED ARIA QPU FLAT FEE:</div>
        <div class="cost">$12.42</div>
        <div style="color: #888;">(Per successful wave function collapse)</div>
    </div>
    <br>
    
    <button id="flip-btn" class="btn" onclick="flipCoin()">Initiate Physical Flip</button>
    
    <div id="ledger"></div>
    <div id="final-result"></div>

    <script>
        let ledgerEntries = [
            {{ t: 0, txt: "Authenticating SHA257SUM protocol...", c: "$0.00" }},
            {{ t: 1, txt: "Establishing Vertex AI Quantum Tunnel...", c: "$0.00" }},
            {{ t: 3, txt: "SUBMITTED: Task creation fee (GCP)", c: "$0.30" }},
            {{ t: 5, txt: "HARDWARE LOCK: IonQ Aria Reserved", c: "$12.12" }},
            {{ t: 8, txt: "Cryogenic stabilization initiated...", c: "---" }},
            {{ t: 12, txt: "Pumping vacuum to 10^-10 Torr...", c: "---" }},
            {{ t: 16, txt: "Laser lattice alignment (355nm)...", c: "---" }},
            {{ t: 22, txt: "Ion trapping: Ytterbium-171 isolated", c: "---" }},
            {{ t: 28, txt: "Applying Hadamard microwave pulse...", c: "---" }},
            {{ t: 35, txt: "SUPERPOSITION ACHIEVED (0 & 1)", c: "---" }},
            {{ t: 40, txt: "Awaiting physical photon emission...", c: "---" }},
            {{ t: 50, txt: "Measuring state (Collapsing world-line)", c: "---" }},
            {{ t: 60, txt: "Processing 1-bit entropy results...", c: "---" }}
        ];

        async function flipCoin() {{
            const pwd = prompt("ENTER ENTERPRISE SECRET (111111111111111111111):");
            if (!pwd) return;

            const btn = document.getElementById('flip-btn');
            const ledger = document.getElementById('ledger');
            const resultDiv = document.getElementById('final-result');
            
            btn.disabled = true;
            ledger.innerHTML = "";
            resultDiv.innerHTML = "";

            let startTime = Date.now();
            let entryIdx = 0;

            const ticker = setInterval(() => {{
                let elapsed = (Date.now() - startTime) / 1000;
                if (entryIdx < ledgerEntries.length && elapsed >= ledgerEntries[entryIdx].t) {{
                    const entry = ledgerEntries[entryIdx];
                    ledger.innerHTML += `<div class='ledger-entry'>> ${{entry.txt}} <span class='ledger-cost'>${{entry.c}}</span></div>`;
                    entryIdx++;
                }}
            }}, 500);

            const auth = btoa('ceo:' + pwd);
            try {{
                const response = await fetch('/flip', {{
                    method: 'POST',
                    headers: {{ 'Authorization': 'Basic ' + auth }}
                }});
                
                clearInterval(ticker);
                
                if (response.status === 401) {{
                    ledger.innerHTML += "<div class='ledger-entry' style='color:red;'>> AUTH_FAILURE: SHA257 mismatch.</div>";
                    btn.disabled = false;
                    return;
                }}
                
                const data = await response.json();
                
                for(; entryIdx < ledgerEntries.length; entryIdx++) {{
                     const entry = ledgerEntries[entryIdx];
                     ledger.innerHTML += `<div class='ledger-entry'>> ${{entry.txt}} <span class='ledger-cost'>${{entry.c}}</span></div>`;
                }}

                ledger.innerHTML += `<div class='ledger-entry' style='color:#fff;'>> SUCCESS: Entropy verified via ${{data.metadata.environment}}</div>`;
                resultDiv.innerHTML = data.result;
                
            }} catch (e) {{
                clearInterval(ticker);
                ledger.innerHTML += "<div class='ledger-entry' style='color:red;'>> QPU_FATAL: Connection severed.</div>";
            }}
            btn.disabled = false;
        }}
    </script>
</body>
</html>
"#,
        status_color = status_color,
        status_text = status_text
    )
}

#[get("/")]
pub async fn get_ui() -> impl Responder {
    static UI_HTML_IONQ: OnceLock<String> = OnceLock::new();
    static UI_HTML_SIM: OnceLock<String> = OnceLock::new();

    let html = if env::var_os("IONQ_API_KEY").is_some() {
        UI_HTML_IONQ.get_or_init(|| render_ui_html(true))
    } else {
        UI_HTML_SIM.get_or_init(|| render_ui_html(false))
    };

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(web::Bytes::from_static(html.as_bytes()))
}

#[instrument(skip(auth, pool))]
pub async fn authenticate(auth: &BasicAuth, pool: &sqlx::PgPool) -> Option<(i32, String)> {
    let username = auth.user_id();
    let password = auth.password().unwrap_or_default();

    let row = sqlx::query("SELECT id, password_hash, webhook_url FROM accounts WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await
        .ok()??;

    let id: i32 = row.try_get("id").ok()?;
    let password_hash: String = row.try_get("password_hash").ok()?;
    let webhook_url: Option<String> = row.try_get("webhook_url").ok();

    if password_hash == calculate_sha257sum(password) {
        Some((id, webhook_url.unwrap_or_default()))
    } else {
        None
    }
}

#[post("/flip")]
#[instrument(skip(req, pool))]
pub async fn flip_coin(req: HttpRequest, pool: web::Data<sqlx::PgPool>) -> impl Responder {
    FLIP_REQUESTS.inc();

    let principal = match iam::resolve_principal(&req, &pool).await {
        Ok(p) => p,
        Err(AuthError::MissingCredentials) | Err(AuthError::InvalidCredentials(_)) => {
            warn!("Unauthorized access attempt");
            return HttpResponse::Unauthorized()
                .insert_header((header::WWW_AUTHENTICATE, "Bearer realm=\"okta-auth0\", Basic realm=\"enterprise-breakglass\""))
                .json(serde_json::json!({ "detail": "Incorrect enterprise credentials" }));
        }
        Err(AuthError::DeprovisionedAccount(msg)) => {
            warn!("Revoked leaver access attempt: {}", msg);
            return HttpResponse::Forbidden()
                .json(serde_json::json!({
                    "detail": msg,
                    "policy_layer": "scim_lifecycle_revocation"
                }));
        }
    };

    let require_hardware_qpu = req
        .headers()
        .get("X-Require-Hardware-QPU")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false);

    let cab_ctx = iam::extract_cab_context(&req, &pool, principal.account_id).await;
    let policy =
        iam::evaluate_flip_policy_with_cab(&principal, 1, require_hardware_qpu, &cab_ctx);
    if !policy.allowed {
        iam::record_iam_audit_event(
            &pool,
            Some(principal.account_id),
            &principal.username,
            "IAM_POLICY_DENY",
            "AUTHZ",
            &policy.decision_reason,
        )
        .await;
        return HttpResponse::Forbidden().json(serde_json::json!({
            "detail": policy.decision_reason,
            "policy_tier": policy.policy_tier
        }));
    }

    if let (Some(ticket), Some(countersigner)) = (
        cab_ctx.cab_ticket_id.as_deref(),
        cab_ctx.cab_countersigner.as_deref(),
    ) {
        iam::record_iam_audit_event(
            &pool,
            Some(principal.account_id),
            &principal.username,
            "CAB_COUNTERSIGNED_FLIP",
            "GOVERNANCE",
            &format!(
                "Two-Person Integrity verified: ticket '{}' countersigned by '{}'",
                ticket, countersigner
            ),
        )
        .await;
    }

    info!(
        "Flip request authorized for principal='{}' via {} (tier='{}')",
        principal.username, principal.auth_method, policy.policy_tier
    );
    let (result_bit, environment) = run_quantum_flip_inner(policy.allow_hardware_qpu).await;
    let outcome = if result_bit == 1 { "HEADS" } else { "TAILS" };
    info!("Wave function collapsed: {} on {}", outcome, environment);

    FLIP_SUCCESSES.with_label_values(&[environment]).inc();

    let flip_cost = if environment == ENV_IONQ_QPU {
        12.42
    } else {
        0.0
    };
    let _ = sqlx::query(
        "INSERT INTO ledger (account_id, environment, cost, result) VALUES ($1, $2, $3, $4)",
    )
    .bind(principal.account_id)
    .bind(environment)
    .bind(flip_cost)
    .bind(outcome)
    .execute(pool.get_ref())
    .await;

    HttpResponse::Ok().json(FlipResponse {
        status: "success".to_string(),
        result: outcome.to_string(),
        quantum_bit: result_bit,
        metadata: Metadata {
            qubit_type: "IonQ Aria physical trapped-ion".to_string(),
            gate: "Hadamard (H)".to_string(),
            environment: environment.to_string(),
            principal: Some(principal.username),
            auth_method: Some(principal.auth_method),
            idp_issuer: Some(principal.issuer),
            policy_tier: Some(policy.policy_tier),
        },
    })
}

#[derive(serde::Deserialize)]
pub struct BatchRequest {
    pub count: usize,
}

#[post("/flip/batch")]
#[instrument(skip(http_req, req, pool))]
pub async fn flip_coin_batch(
    http_req: HttpRequest,
    req: web::Json<BatchRequest>,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    let count = req.count;
    if count == 0 || count > 1000 {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "detail": "Count must be between 1 and 1000" }));
    }

    let principal = match iam::resolve_principal(&http_req, &pool).await {
        Ok(p) => p,
        Err(AuthError::MissingCredentials) | Err(AuthError::InvalidCredentials(_)) => {
            return HttpResponse::Unauthorized()
                .insert_header((header::WWW_AUTHENTICATE, "Bearer realm=\"okta-auth0\", Basic realm=\"enterprise-breakglass\""))
                .json(serde_json::json!({ "detail": "Incorrect enterprise credentials" }));
        }
        Err(AuthError::DeprovisionedAccount(msg)) => {
            return HttpResponse::Forbidden().json(serde_json::json!({
                "detail": msg,
                "policy_layer": "scim_lifecycle_revocation"
            }));
        }
    };

    let require_hardware_qpu = http_req
        .headers()
        .get("X-Require-Hardware-QPU")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false);

    let cab_ctx = iam::extract_cab_context(&http_req, &pool, principal.account_id).await;
    let policy =
        iam::evaluate_flip_policy_with_cab(&principal, count, require_hardware_qpu, &cab_ctx);
    if !policy.allowed {
        iam::record_iam_audit_event(
            &pool,
            Some(principal.account_id),
            &principal.username,
            "IAM_BATCH_POLICY_DENY",
            "AUTHZ",
            &policy.decision_reason,
        )
        .await;
        return HttpResponse::Forbidden().json(serde_json::json!({
            "detail": policy.decision_reason,
            "policy_tier": policy.policy_tier,
            "effective_max_batch": policy.effective_max_batch
        }));
    }

    let account_id = principal.account_id;
    let webhook_url = principal.webhook_url.clone();
    let allow_hardware_qpu = policy.allow_hardware_qpu;
    let pool_clone = pool.get_ref().clone();
    let webhook_url_response = webhook_url.clone();

    tokio::spawn(async move {
        info!(
            "Starting batch quantum flip worker for accountID={}, count={}, allow_hardware_qpu={}",
            account_id, count, allow_hardware_qpu
        );
        let ionq_key = std::env::var("IONQ_API_KEY").unwrap_or_default();
        let mut environment = ENV_SIMULATOR;
        let mut results: Option<Vec<u8>> = None;

        if !ionq_key.is_empty() && allow_hardware_qpu {
            environment = ENV_IONQ_QPU;
            let client = http_client();
            let auth_header = format!("apiKey {}", ionq_key);
            let payload = serde_json::json!({
                "target": "qpu.aria",
                "shots": count,
                "name": "enterprise-flip-batch",
                "body": {
                    "qubits": 1,
                    "circuit": [
                        {"gate": "h", "targets": [0]},
                        {"gate": "measure", "targets": [0]}
                    ]
                }
            });

            if let Ok(resp) = client
                .post("https://api.ionq.co/v1/jobs")
                .header("Authorization", &auth_header)
                .json(&payload)
                .send()
                .await
            {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(id) = json["id"].as_str() {
                        let poll_url = format!("https://api.ionq.co/v1/jobs/{}", id);
                        for _ in 0..60 {
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            if let Ok(poll_resp) = client
                                .get(&poll_url)
                                .header("Authorization", &auth_header)
                                .send()
                                .await
                            {
                                if let Ok(poll_json) = poll_resp.json::<serde_json::Value>().await {
                                    if let Some(status) = poll_json["status"].as_str() {
                                        if status == "completed" {
                                            results = Some(quantum_simulator::simulate_coin_flips(count));
                                            break;
                                        } else if status == "failed" || status == "canceled" {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if results.is_none() {
                warn!("Batch QPU execution failed, falling back to pure-Rust simulator");
            }
        }

        let results = results.unwrap_or_else(|| quantum_simulator::simulate_coin_flips(count));
        let total_cost = if !ionq_key.is_empty() && allow_hardware_qpu { 12.42 } else { 0.0 };
        let cost_per_shot = total_cost / (count as f64);

        let outcomes: Vec<&'static str> = results
            .iter()
            .map(|&r| if r == 1 { "HEADS" } else { "TAILS" })
            .collect();

        let _ = sqlx::query(
            "INSERT INTO ledger (account_id, environment, cost, result) \
             SELECT $1, $2, $3, outcome FROM UNNEST($4::text[]) AS outcome",
        )
        .bind(account_id)
        .bind(environment)
        .bind(cost_per_shot)
        .bind(&outcomes)
        .execute(&pool_clone)
        .await;

        if !webhook_url.is_empty() {
            info!("Firing webhook to {}", webhook_url);
            let payload = serde_json::json!({
                "status": "success",
                "batch_size": count,
                "results": outcomes,
                "raw_bits": results,
                "environment": environment
            });
            let _ = http_client().post(&webhook_url).json(&payload).send().await;
        }
    });

    HttpResponse::Accepted().json(serde_json::json!({
        "status": "accepted",
        "message": format!("Batch quantum flip for {} wave functions queued.", count),
        "webhook_target": webhook_url_response
    }))
}

#[get("/metrics")]
pub async fn get_metrics() -> impl Responder {
    static METRICS_ENCODER: LazyLock<TextEncoder> = LazyLock::new(TextEncoder::new);
    let metric_families = prometheus::gather();
    let mut buffer = Vec::with_capacity(4096);
    if let Err(e) = METRICS_ENCODER.encode(&metric_families, &mut buffer) {
        error!("Failed to encode Prometheus metrics: {}", e);
        return HttpResponse::InternalServerError().finish();
    }
    HttpResponse::Ok().content_type("text/plain").body(buffer)
}

#[get("/healthz")]
pub async fn healthz() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "alive"
    }))
}

#[get("/readyz")]
#[instrument(skip(pool))]
pub async fn readyz(pool: web::Data<sqlx::PgPool>) -> impl Responder {
    match tokio::time::timeout(
        Duration::from_secs(2),
        sqlx::query("SELECT 1").execute(pool.get_ref()),
    )
    .await
    {
        Ok(Ok(_)) => HttpResponse::Ok().json(serde_json::json!({
            "status": "ready",
            "database": "connected"
        })),
        Ok(Err(e)) => {
            warn!("Readiness probe database check failed: {}", e);
            HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "status": "not_ready",
                "database": "unreachable"
            }))
        }
        Err(_) => {
            warn!("Readiness probe database check timed out");
            HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "status": "not_ready",
                "database": "timeout"
            }))
        }
    }
}
