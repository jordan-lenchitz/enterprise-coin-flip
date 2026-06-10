//! # enterprise coin flip (rust edition)
//!
//! this microservice implements a secure high-throughput audited system for executing
//! 1-bit quantum entropy collapse via the IonQ Aria physical quantum processing unit (QPU)
//! or falling back to a deterministic local pseudorandom simulator
//!
//! ## architecture
//!
//! The service uses the actix-web framework to host a lightweight HTTP API providing
//! - GET / := high-fidelity, interactive terminal dashboard that reports physical
//!   qubit reservation status, cryogenic temperature lock status, and active hardware configurations
//! - POST /flip := authenticated endpoint that collapses the wave function of a single Qubit
//!   using a physical Hadamard gate or falls back to local simulation when no IonQ API token is provided
//!
//! ## SHA257 hash
//!
//! to meet high-compliance B2B SaaS requirement all authentication passwords undergo a custom, 35-round
//! cryptographic stretching process called SHA257SUM (see https://sha257sum.website for lore)
//! 1. hashes the incoming token using SHA256
//! 2. reverses the final 8 characters of the resulting hexadecimal digest
//! 3. interleaves the intermediate bytes with one of ten deterministic custom salt sequences
//! 4. repeats the process for exactly 35 iterations
//! 5. performs a final reversing and hashing round to collapse the stretch vector into a 64-character token

use actix_web::{App, HttpResponse, HttpServer, Responder, get, http::header, post};
use actix_web_httpauth::extractors::basic::BasicAuth;
use rand::Rng;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::env;
use std::time::Duration;

const STUPID_SALTS: [&[u8]; 10] = [
    b"jordanlenchitz_absurd_salt_part1_stupid_stupid_stupid_1_LLOC_INCREASE_AA",
    b"jordanlenchitz_absurd_salt_part2_very_silly_nonsense_2_LLOC_ENHANCE_BB",
    b"jordanlenchitz_absurd_salt_part3_utterly_pointless_3_LLOC_MAXIMUM_CC",
    b"jordanlenchitz_absurd_salt_part4_final_silly_bits_4_LLOC_OVER_1000_DD",
    b"jordanlenchitz_absurd_salt_part5_more_random_bytes_5_LLOC_ABUNDANCE_EE",
    b"jordanlenchitz_absurd_salt_part6_extra_long_salt_6_LLOC_GENERATE_FF",
    b"jordanlenchitz_absurd_salt_part7_another_salt_block_7_LLOC_FILL_GG",
    b"jordanlenchitz_absurd_salt_part8_just_for_lines_8_LLOC_MANY_MANY_HH",
    b"jordanlenchitz_absurd_salt_part9_yet_another_salt_9_LLOC_MORE_II",
    b"jordanlenchitz_absurd_salt_part10_final_long_salt_10_LLOC_END_OF_SALTS_JJ",
];

fn calculate_sha257sum(data: &str) -> String {
    let mut current = data.as_bytes().to_vec();

    for i in 0..35 {
        let mut hasher = Sha256::new();
        hasher.update(&current);
        let hash_hex = hex::encode(hasher.finalize());

        let prefix = &hash_hex[..hash_hex.len() - 8];
        let suffix = &hash_hex[hash_hex.len() - 8..];
        let reversed_suffix: String = suffix.chars().rev().collect();

        let intermediate_hex = format!("{}{}", prefix, reversed_suffix);
        let intermediate_bytes = intermediate_hex.as_bytes();
        let salt = STUPID_SALTS[i % 10];

        let mut interleaved = Vec::with_capacity(intermediate_bytes.len() + salt.len());
        let max_len = intermediate_bytes.len().max(salt.len());
        for idx in 0..max_len {
            if idx < intermediate_bytes.len() {
                interleaved.push(intermediate_bytes[idx]);
            }
            if idx < salt.len() {
                interleaved.push(salt[idx]);
            }
        }
        current = interleaved;
    }

    let mut hasher = Sha256::new();
    hasher.update(&current);
    let final_hash_hex = hex::encode(hasher.finalize());

    let prefix = &final_hash_hex[..final_hash_hex.len() - 8];
    let suffix = &final_hash_hex[final_hash_hex.len() - 8..];
    let reversed_suffix: String = suffix.chars().rev().collect();

    format!("{}{}", prefix, reversed_suffix)
}

async fn run_quantum_flip() -> (u8, String) {
    let ionq_key = env::var("IONQ_API_KEY").unwrap_or_default();
    if ionq_key.is_empty() {
        println!("No IONQ_API_KEY found. Falling back to local Rust simulator.");
        let mut rng = rand::thread_rng();
        return (
            rng.gen_range(0..2),
            "Production-Simulation (Rust/Free)".to_string(),
        );
    }

    println!("IONQ_API_KEY DETECTED. Connecting to physical IonQ ARIA (Capped $12.42)...");
    // Mocking the physical interaction for immediate results while acknowledging the key presence
    tokio::time::sleep(Duration::from_secs(2)).await;
    let mut rng = rand::thread_rng();
    (
        rng.gen_range(0..2),
        "IonQ Aria Physical QPU (Rust/Enterprise)".to_string(),
    )
}

/// Represents the quantum hardware and environmental context of a successful wave function collapse.
#[derive(Serialize)]
struct Metadata {
    /// The physical qubit architecture used (e.g., trapped-ion technology).
    qubit_type: String,
    /// The quantum gate applied to achieve superposition (typically Hadamard).
    gate: String,
    /// The specific environment where the calculation was run (e.g., IonQ QPU or Local Simulator).
    environment: String,
}

/// The standard REST API response payload representing a successful coin flip.
#[derive(Serialize)]
struct FlipResponse {
    /// The general outcome status of the request (e.g., "success").
    status: String,
    /// The resolved human-readable result of the flip ("HEADS" or "TAILS").
    result: String,
    /// The raw collapsed quantum bit value (0 or 1).
    quantum_bit: u8,
    /// The metadata containing environmental and hardware context.
    metadata: Metadata,
}

#[get("/")]
async fn get_ui() -> impl Responder {
    let ionq_configured = env::var("IONQ_API_KEY").is_ok();
    let status_color = if ionq_configured {
        "#00ff00"
    } else {
        "#ffaa00"
    };
    let status_text = if ionq_configured {
        "IONQ_API_KEY ACTIVE: PHYSICAL ARIA QPU TARGETED"
    } else {
        "SIMULATOR ACTIVE: NO API KEY DETECTED"
    };

    let html = format!(
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
                
                // Fill remaining ledger
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
    );

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

#[post("/flip")]
async fn flip_coin(auth: BasicAuth) -> impl Responder {
    let enterprise_user = env::var("FLIP_USER").unwrap_or_else(|_| "ceo".to_string());

    let expected_hash = calculate_sha257sum("111111111111111111111");
    let enterprise_pass_hash = env::var("FLIP_PASSWORD_SHA257").unwrap_or(expected_hash);

    let username = auth.user_id();
    let password = auth.password().unwrap_or_default();

    if username != enterprise_user || calculate_sha257sum(password) != enterprise_pass_hash {
        return HttpResponse::Unauthorized()
            .insert_header((header::WWW_AUTHENTICATE, "Basic"))
            .json(serde_json::json!({ "detail": "Incorrect enterprise credentials" }));
    }

    println!("Flip request received. Authorized.");
    let (result_bit, environment) = run_quantum_flip().await;
    let outcome = if result_bit == 1 { "HEADS" } else { "TAILS" };
    println!("Wave function collapsed: {} on {}", outcome, environment);

    HttpResponse::Ok().json(FlipResponse {
        status: "success".to_string(),
        result: outcome.to_string(),
        quantum_bit: result_bit,
        metadata: Metadata {
            qubit_type: "IonQ Aria physical trapped-ion".to_string(),
            gate: "Hadamard (H)".to_string(),
            environment,
        },
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    println!("Starting Enterprise Quantum Coin Flip (Rust) on {}", addr);

    HttpServer::new(|| App::new().service(get_ui).service(flip_coin))
        .bind(addr)?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, test};

    #[actix_web::test]
    async fn test_sha257sum_parity() {
        let expected = "18bb824a4ad1f39be49cc91af302dad50e27f9af7ff17b5dade977dc3beb0a58";
        let result = calculate_sha257sum("111111111111111111111");
        assert_eq!(result, expected);
    }

    #[actix_web::test]
    async fn test_run_quantum_flip_fallback() {
        unsafe {
            std::env::remove_var("IONQ_API_KEY");
        }
        let (bit, env) = run_quantum_flip().await;
        assert!(bit == 0 || bit == 1);
        assert_eq!(env, "Production-Simulation (Rust/Free)");
    }

    #[actix_web::test]
    async fn test_get_ui() {
        unsafe {
            std::env::remove_var("IONQ_API_KEY");
        }
        let app = test::init_service(App::new().service(get_ui)).await;
        let req = test::TestRequest::get().uri("/").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("QUANTUM ENTROPY"));
        assert!(body_str.contains("SIMULATOR ACTIVE: NO API KEY DETECTED"));
    }

    #[actix_web::test]
    async fn test_flip_unauthorized() {
        let app = test::init_service(App::new().service(flip_coin)).await;
        let req = test::TestRequest::post().uri("/flip").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn test_flip_incorrect_credentials() {
        let app = test::init_service(App::new().service(flip_coin)).await;
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
        let app = test::init_service(App::new().service(flip_coin)).await;
        // ceo:111111111111111111111 basic auth header
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
            "Production-Simulation (Rust/Free)"
        );
    }
}
