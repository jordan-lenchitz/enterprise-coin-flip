use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use lazy_static::lazy_static;
use prometheus::{register_int_counter_vec, IntCounterVec};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, instrument, warn};

use crate::crypto::calculate_sha257sum;

lazy_static! {
    pub static ref IAM_AUTHN_TOTAL: IntCounterVec = register_int_counter_vec!(
        "iam_authn_requests_total",
        "Total authentication attempts by method, identity provider issuer, and outcome",
        &["method", "issuer", "outcome"]
    )
    .unwrap();
    pub static ref IAM_AUTHZ_DECISIONS: IntCounterVec = register_int_counter_vec!(
        "iam_authz_decisions_total",
        "Total RBAC and ABAC authorization policy decisions",
        &["policy_layer", "decision", "reason"]
    )
    .unwrap();
    pub static ref SCIM_EVENTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "scim_provisioning_events_total",
        "Total SCIM 2.0 lifecycle management provisioning events (Joiner, Mover, Leaver)",
        &["operation", "lifecycle_stage"]
    )
    .unwrap();
    pub static ref OIG_CERTIFICATIONS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "governance_certifications_total",
        "Total Okta Identity Governance (OIG) access review certification decisions",
        &["decision"]
    )
    .unwrap();
}

pub const DEFAULT_OKTA_ISSUER: &str = "https://jordanlenchitz.okta.com/oauth2/default";
pub const DEFAULT_AUTH0_ISSUER: &str = "https://identity.jordanlenchitz.com/";
pub const DEFAULT_LOCAL_ISSUER: &str = "http://localhost:8080/oauth2/default";
pub const DEFAULT_AUDIENCE: &str = "api://quantum-coin-flip";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcClaims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: u64,
    pub iat: u64,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default)]
    pub scp: Vec<String>,
    #[serde(default)]
    pub amr: Vec<String>,
    #[serde(default)]
    pub department: Option<String>,
    #[serde(default)]
    pub cost_center: Option<String>,
    #[serde(default)]
    pub org_id: Option<String>,
    #[serde(default)]
    pub qpu_entitled: Option<bool>,
    #[serde(default)]
    pub max_batch_shots: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthenticatedPrincipal {
    pub account_id: i32,
    pub username: String,
    pub email: String,
    pub auth_method: String,
    pub issuer: String,
    pub roles: Vec<String>,
    pub scopes: Vec<String>,
    pub amr: Vec<String>,
    pub department: String,
    pub cost_center: String,
    pub org_id: Option<String>,
    pub qpu_entitled: bool,
    pub max_batch_shots: usize,
    pub webhook_url: String,
}

impl AuthenticatedPrincipal {
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r.eq_ignore_ascii_case(role))
    }

    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s.eq_ignore_ascii_case(scope))
    }

    pub fn has_mfa(&self) -> bool {
        self.auth_method == "basic-breakglass"
            || self.amr.iter().any(|m| {
                matches!(
                    m.to_ascii_lowercase().as_str(),
                    "mfa" | "otp" | "hwk" | "webauthn" | "push"
                )
            })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PolicyEvaluation {
    pub allowed: bool,
    pub allow_hardware_qpu: bool,
    pub policy_tier: String,
    pub decision_reason: String,
    pub effective_max_batch: usize,
}

fn jwt_signing_secret() -> String {
    env::var("OIDC_JWT_SECRET")
        .unwrap_or_else(|_| "enterprise-quantum-oidc-hmac-secret-2026".to_string())
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;
    let mut key_block = [0u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        let mut hasher = Sha256::new();
        hasher.update(key);
        let digest = hasher.finalize();
        key_block[..32].copy_from_slice(&digest);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; BLOCK_SIZE];
    let mut opad = [0x5cu8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        ipad[i] ^= key_block[i];
        opad[i] ^= key_block[i];
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);
    let outer_hash = outer.finalize();

    let mut out = [0u8; 32];
    out.copy_from_slice(&outer_hash);
    out
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub fn mint_jwt(claims: &OidcClaims) -> String {
    let header = serde_json::json!({
        "alg": "HS256",
        "typ": "JWT",
        "kid": "jordanlenchitz-iam-key-2026-01"
    });
    let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap());
    let claims_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).unwrap());
    let signing_input = format!("{}.{}", header_b64, claims_b64);
    let sig = hmac_sha256(jwt_signing_secret().as_bytes(), signing_input.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig);
    format!("{}.{}", signing_input, sig_b64)
}

pub fn verify_jwt(token: &str) -> Result<OidcClaims, String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Malformed JWT: expected 3 dot-separated segments".to_string());
    }

    let header_bytes = URL_SAFE_NO_PAD
        .decode(parts[0])
        .map_err(|_| "Malformed JWT header base64url".to_string())?;
    let header_json: serde_json::Value = serde_json::from_slice(&header_bytes)
        .map_err(|_| "Malformed JWT header JSON".to_string())?;

    if header_json["alg"].as_str() != Some("HS256") {
        return Err("Unsupported JWT signing algorithm; expected HS256".to_string());
    }

    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let expected_sig = hmac_sha256(jwt_signing_secret().as_bytes(), signing_input.as_bytes());
    let provided_sig = URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|_| "Malformed JWT signature base64url".to_string())?;

    if !constant_time_eq(&expected_sig, &provided_sig) {
        return Err("Invalid JWT cryptographic signature".to_string());
    }

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| "Malformed JWT payload base64url".to_string())?;
    let claims: OidcClaims = serde_json::from_slice(&payload_bytes)
        .map_err(|e| format!("Invalid OIDC claims schema: {}", e))?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if claims.exp <= now {
        return Err("OIDC access token has expired (exp <= now)".to_string());
    }

    let custom_issuer = env::var("OIDC_ISSUER_URI").unwrap_or_default();
    let trusted_issuers = [
        DEFAULT_OKTA_ISSUER,
        DEFAULT_AUTH0_ISSUER,
        DEFAULT_LOCAL_ISSUER,
        custom_issuer.as_str(),
    ];
    if !trusted_issuers
        .iter()
        .any(|&iss| !iss.is_empty() && iss == claims.iss)
    {
        return Err(format!("Untrusted OIDC issuer: {}", claims.iss));
    }

    if claims.aud != DEFAULT_AUDIENCE {
        return Err(format!(
            "Invalid OIDC audience '{}'; expected '{}'",
            claims.aud, DEFAULT_AUDIENCE
        ));
    }

    Ok(claims)
}

#[derive(Debug)]
pub enum AuthError {
    MissingCredentials,
    InvalidCredentials(String),
    DeprovisionedAccount(String),
}

#[instrument(skip(req, pool))]
pub async fn resolve_principal(
    req: &HttpRequest,
    pool: &sqlx::PgPool,
) -> Result<AuthenticatedPrincipal, AuthError> {
    let auth_header = req
        .headers()
        .get(actix_web::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AuthError::MissingCredentials)?;

    if let Some(bearer_token) = auth_header.strip_prefix("Bearer ") {
        let claims = match verify_jwt(bearer_token.trim()) {
            Ok(c) => c,
            Err(e) => {
                IAM_AUTHN_TOTAL
                    .with_label_values(&["oidc_bearer", "unknown", "failure"])
                    .inc();
                warn!("OIDC JWT verification failed: {}", e);
                return Err(AuthError::InvalidCredentials(e));
            }
        };

        let row = sqlx::query(
            "SELECT id, username, email, active, department, cost_center, roles, qpu_entitled, max_batch_shots, webhook_url \
             FROM accounts WHERE username = $1 OR email = $1 OR external_id = $1 LIMIT 1",
        )
        .bind(&claims.sub)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        if let Some(r) = row {
            let active: bool = r.try_get("active").unwrap_or(true);
            let username: String = r.try_get("username").unwrap_or_else(|_| claims.sub.clone());
            if !active {
                IAM_AUTHN_TOTAL
                    .with_label_values(&["oidc_bearer", &claims.iss, "revoked_leaver"])
                    .inc();
                IAM_AUTHZ_DECISIONS
                    .with_label_values(&["scim_lifecycle", "deny", "account_deprovisioned"])
                    .inc();
                warn!(
                    "Blocked OIDC token for SCIM-deprovisioned Leaver account '{}'",
                    username
                );
                return Err(AuthError::DeprovisionedAccount(format!(
                    "Account '{}' has been deprovisioned via Okta SCIM Leaver workflow",
                    username
                )));
            }

            let db_roles: String = r.try_get("roles").unwrap_or_default();
            let mut roles: Vec<String> = db_roles
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            for g in &claims.groups {
                if !roles.iter().any(|existing| existing.eq_ignore_ascii_case(g)) {
                    roles.push(g.clone());
                }
            }

            let db_qpu: bool = r.try_get("qpu_entitled").unwrap_or(false);
            let qpu_entitled = claims.qpu_entitled.unwrap_or(db_qpu);

            let db_max_batch: i32 = r.try_get("max_batch_shots").unwrap_or(100);
            let max_batch_shots = claims
                .max_batch_shots
                .unwrap_or(db_max_batch.max(1) as usize);

            let department = claims
                .department
                .clone()
                .or_else(|| r.try_get::<String, _>("department").ok())
                .unwrap_or_else(|| "Unassigned".to_string());

            let cost_center = claims
                .cost_center
                .clone()
                .or_else(|| r.try_get::<String, _>("cost_center").ok())
                .unwrap_or_default();

            IAM_AUTHN_TOTAL
                .with_label_values(&["oidc_bearer", &claims.iss, "success"])
                .inc();

            return Ok(AuthenticatedPrincipal {
                account_id: r.try_get("id").unwrap_or(0),
                username,
                email: claims
                    .email
                    .or_else(|| r.try_get::<Option<String>, _>("email").ok().flatten())
                    .unwrap_or_else(|| "unknown@jordanlenchitz.com".to_string()),
                auth_method: "oidc_bearer".to_string(),
                issuer: claims.iss,
                roles,
                scopes: claims.scp,
                amr: claims.amr,
                department,
                cost_center,
                org_id: claims.org_id,
                qpu_entitled,
                max_batch_shots,
                webhook_url: r
                    .try_get::<Option<String>, _>("webhook_url")
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
            });
        }

        // Federated OIDC principal without a pre-existing DB row (JIT federated evaluation)
        IAM_AUTHN_TOTAL
            .with_label_values(&["oidc_bearer", &claims.iss, "success"])
            .inc();

        let is_exec = claims
            .groups
            .iter()
            .any(|g| g.eq_ignore_ascii_case("Quantum-Executives"));
        let default_batch = if is_exec { 1000 } else { 50 };

        return Ok(AuthenticatedPrincipal {
            account_id: 1,
            username: claims.sub.clone(),
            email: claims
                .email
                .unwrap_or_else(|| format!("{}@federated.example.com", claims.sub)),
            auth_method: "oidc_bearer".to_string(),
            issuer: claims.iss,
            roles: claims.groups,
            scopes: claims.scp,
            amr: claims.amr,
            department: claims
                .department
                .unwrap_or_else(|| "Federated".to_string()),
            cost_center: claims.cost_center.unwrap_or_default(),
            org_id: claims.org_id,
            qpu_entitled: claims.qpu_entitled.unwrap_or(is_exec),
            max_batch_shots: claims.max_batch_shots.unwrap_or(default_batch),
            webhook_url: String::new(),
        });
    }

    if let Some(basic_b64) = auth_header.strip_prefix("Basic ") {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(basic_b64.trim())
            .map_err(|_| {
                AuthError::InvalidCredentials("Invalid Basic Auth base64 encoding".to_string())
            })?;
        let cred_str = String::from_utf8(decoded).map_err(|_| {
            AuthError::InvalidCredentials("Invalid Basic Auth UTF-8 string".to_string())
        })?;
        let (username, password) = cred_str
            .split_once(':')
            .unwrap_or((cred_str.as_str(), ""));

        let row = sqlx::query(
            "SELECT id, password_hash, webhook_url, email, active, department, cost_center, roles, qpu_entitled, max_batch_shots, idp_issuer \
             FROM accounts WHERE username = $1",
        )
        .bind(username)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .ok_or_else(|| {
            IAM_AUTHN_TOTAL
                .with_label_values(&["basic_breakglass", "local-breakglass", "failure"])
                .inc();
            AuthError::InvalidCredentials("Unknown enterprise account".to_string())
        })?;

        let password_hash: String = row.try_get("password_hash").unwrap_or_default();
        if password_hash != calculate_sha257sum(password) {
            IAM_AUTHN_TOTAL
                .with_label_values(&["basic_breakglass", "local-breakglass", "failure"])
                .inc();
            return Err(AuthError::InvalidCredentials(
                "Incorrect enterprise credentials".to_string(),
            ));
        }

        let active: bool = row.try_get("active").unwrap_or(true);
        if !active {
            IAM_AUTHN_TOTAL
                .with_label_values(&["basic_breakglass", "local-breakglass", "revoked_leaver"])
                .inc();
            IAM_AUTHZ_DECISIONS
                .with_label_values(&["scim_lifecycle", "deny", "account_deprovisioned"])
                .inc();
            return Err(AuthError::DeprovisionedAccount(format!(
                "Account '{}' has been deprovisioned via SCIM Leaver workflow",
                username
            )));
        }

        let roles_raw: String = row
            .try_get("roles")
            .unwrap_or_else(|_| "Quantum-Executives".to_string());
        let roles: Vec<String> = roles_raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        IAM_AUTHN_TOTAL
            .with_label_values(&["basic_breakglass", "local-breakglass", "success"])
            .inc();

        return Ok(AuthenticatedPrincipal {
            account_id: row.try_get("id").unwrap_or(1),
            username: username.to_string(),
            email: row
                .try_get::<Option<String>, _>("email")
                .ok()
                .flatten()
                .unwrap_or_else(|| format!("{}@jordanlenchitz.com", username)),
            auth_method: "basic-breakglass".to_string(),
            issuer: row
                .try_get::<String, _>("idp_issuer")
                .unwrap_or_else(|_| "local-breakglass".to_string()),
            roles,
            scopes: vec![
                "flip:execute".to_string(),
                "flip:batch".to_string(),
                "qpu:hardware".to_string(),
            ],
            amr: vec!["pwd".to_string(), "mfa".to_string()],
            department: row
                .try_get::<String, _>("department")
                .unwrap_or_else(|_| "Executive".to_string()),
            cost_center: row
                .try_get::<String, _>("cost_center")
                .unwrap_or_else(|_| "CC-1001-EXEC".to_string()),
            org_id: None,
            qpu_entitled: row.try_get("qpu_entitled").unwrap_or(true),
            max_batch_shots: row.try_get::<i32, _>("max_batch_shots").unwrap_or(1000) as usize,
            webhook_url: row
                .try_get::<Option<String>, _>("webhook_url")
                .ok()
                .flatten()
                .unwrap_or_default(),
        });
    }

    Err(AuthError::InvalidCredentials(
        "Unsupported Authorization scheme; expected 'Bearer <JWT>' or 'Basic <credentials>'"
            .to_string(),
    ))
}

/// Evaluates Hybrid RBAC + ABAC policy for single and batch quantum flips.
pub fn evaluate_flip_policy(
    principal: &AuthenticatedPrincipal,
    requested_shots: usize,
    require_hardware_qpu: bool,
) -> PolicyEvaluation {
    // 1. RBAC Role Check
    let has_exec_role = principal.has_role("Quantum-Executives");
    let has_analyst_role = principal.has_role("Quantum-Analysts");
    let has_b2b_role = principal.has_role("CIAM-B2B-Partner");

    if !has_exec_role && !has_analyst_role && !has_b2b_role {
        IAM_AUTHZ_DECISIONS
            .with_label_values(&["rbac", "deny", "missing_flip_role"])
            .inc();
        return PolicyEvaluation {
            allowed: false,
            allow_hardware_qpu: false,
            policy_tier: "Denied (RBAC)".to_string(),
            decision_reason: format!(
                "RBAC Policy Deny: Principal '{}' (roles={:?}) lacks an entitled operational role (Quantum-Executives, Quantum-Analysts, or CIAM-B2B-Partner)",
                principal.username, principal.roles
            ),
            effective_max_batch: 0,
        };
    }

    // 2. ABAC Batch Shot Limit Check
    if requested_shots > principal.max_batch_shots {
        IAM_AUTHZ_DECISIONS
            .with_label_values(&["abac", "deny", "batch_quota_exceeded"])
            .inc();
        return PolicyEvaluation {
            allowed: false,
            allow_hardware_qpu: false,
            policy_tier: "Denied (ABAC Quota)".to_string(),
            decision_reason: format!(
                "ABAC Policy Deny: Requested {} shots exceeds principal '{}' max_batch_shots entitlement ({})",
                requested_shots, principal.username, principal.max_batch_shots
            ),
            effective_max_batch: principal.max_batch_shots,
        };
    }

    // 3. ABAC Hardware QPU ($12.42) Entitlement + Cost Center + Adaptive MFA Evaluation
    let has_valid_cost_center = !principal.cost_center.trim().is_empty()
        && principal.cost_center != "UNASSIGNED"
        && principal.cost_center != "NONE";
    let mfa_satisfied = if requested_shots > 1 {
        principal.has_mfa()
    } else {
        true
    };

    let allow_hardware_qpu = principal.qpu_entitled
        && has_valid_cost_center
        && mfa_satisfied
        && (has_exec_role || has_analyst_role || (has_b2b_role && principal.org_id.is_some()));

    if require_hardware_qpu && !allow_hardware_qpu {
        let deny_reason = if !principal.qpu_entitled {
            "missing_qpu_entitlement"
        } else if !has_valid_cost_center {
            "missing_cost_center_attribute"
        } else if !mfa_satisfied {
            "adaptive_mfa_step_up_required"
        } else {
            "b2b_org_id_missing"
        };

        IAM_AUTHZ_DECISIONS
            .with_label_values(&["abac", "deny", deny_reason])
            .inc();

        return PolicyEvaluation {
            allowed: false,
            allow_hardware_qpu: false,
            policy_tier: "Denied (ABAC Hardware QPU Gate)".to_string(),
            decision_reason: format!(
                "ABAC Hardware QPU Gate Deny ({}): qpu_entitled={}, cost_center='{}', mfa_verified={}",
                deny_reason,
                principal.qpu_entitled,
                principal.cost_center,
                principal.has_mfa()
            ),
            effective_max_batch: principal.max_batch_shots,
        };
    }

    let policy_tier = if allow_hardware_qpu {
        IAM_AUTHZ_DECISIONS
            .with_label_values(&["hybrid_rbac_abac", "allow", "hardware_qpu_authorized"])
            .inc();
        format!(
            "Hardware-QPU-Authorized (cost_center={})",
            principal.cost_center
        )
    } else {
        IAM_AUTHZ_DECISIONS
            .with_label_values(&["hybrid_rbac_abac", "allow", "simulator_tier_only"])
            .inc();
        "Simulator-Only (ABAC Cost-Control Tier)".to_string()
    };

    PolicyEvaluation {
        allowed: true,
        allow_hardware_qpu,
        policy_tier,
        decision_reason: "Authorized via Hybrid RBAC+ABAC policy engine".to_string(),
        effective_max_batch: principal.max_batch_shots,
    }
}

pub async fn record_iam_audit_event(
    pool: &sqlx::PgPool,
    account_id: Option<i32>,
    actor: &str,
    event_type: &str,
    lifecycle_stage: &str,
    details: &str,
) {
    let _ = sqlx::query(
        "INSERT INTO iam_audit_log (account_id, actor, event_type, lifecycle_stage, details) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(account_id)
    .bind(actor)
    .bind(event_type)
    .bind(lifecycle_stage)
    .bind(details)
    .execute(pool)
    .await;
}

#[derive(Debug, Deserialize)]
pub struct TokenIssueRequest {
    /// Preset persona ('executive', 'analyst', 'b2b_partner', 'scim_admin', 'governance_admin') or custom subject
    #[serde(default)]
    pub persona: Option<String>,
    #[serde(default)]
    pub sub: Option<String>,
    #[serde(default)]
    pub issuer: Option<String>,
    #[serde(default)]
    pub groups: Option<Vec<String>>,
    #[serde(default)]
    pub cost_center: Option<String>,
    #[serde(default)]
    pub department: Option<String>,
    #[serde(default)]
    pub org_id: Option<String>,
    #[serde(default)]
    pub qpu_entitled: Option<bool>,
    #[serde(default)]
    pub max_batch_shots: Option<usize>,
    #[serde(default)]
    pub amr: Option<Vec<String>>,
}

#[post("/oauth2/token")]
#[instrument(skip(req))]
pub async fn issue_demo_oidc_token(req: web::Json<TokenIssueRequest>) -> impl Responder {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let exp = now + 3600;

    let persona = req
        .persona
        .as_deref()
        .unwrap_or("executive")
        .to_ascii_lowercase();

    let (default_sub, default_iss, default_groups, default_scp, default_amr, default_dept, default_cc, default_org, default_qpu, default_batch) =
        match persona.as_str() {
            "analyst" => (
                "analyst.actuarial",
                DEFAULT_OKTA_ISSUER,
                vec!["Quantum-Analysts".to_string()],
                vec!["flip:execute".to_string(), "flip:batch".to_string()],
                vec!["pwd".to_string()],
                "Actuarial Science",
                "",
                None,
                false,
                50usize,
            ),
            "b2b_partner" => (
                "partner.reinsurance",
                DEFAULT_AUTH0_ISSUER,
                vec!["CIAM-B2B-Partner".to_string()],
                vec!["flip:execute".to_string(), "flip:batch".to_string()],
                vec!["pwd".to_string(), "mfa".to_string()],
                "External Reinsurance Partner",
                "CC-8800-B2B",
                Some("org_jordanlenchitz_reinsure_01".to_string()),
                true,
                250usize,
            ),
            "scim_admin" => (
                "okta.lifecycle.worker",
                DEFAULT_OKTA_ISSUER,
                vec![
                    "SCIM-Provisioner".to_string(),
                    "IAM-Governance-Admins".to_string(),
                ],
                vec!["scim:manage".to_string(), "governance:review".to_string()],
                vec!["hwk".to_string(), "mfa".to_string()],
                "Identity & Access Management",
                "CC-4010-IAM",
                None,
                false,
                10usize,
            ),
            _ => (
                "ceo",
                DEFAULT_OKTA_ISSUER,
                vec![
                    "Quantum-Executives".to_string(),
                    "IAM-Governance-Admins".to_string(),
                ],
                vec![
                    "flip:execute".to_string(),
                    "flip:batch".to_string(),
                    "qpu:hardware".to_string(),
                    "governance:review".to_string(),
                ],
                vec!["pwd".to_string(), "mfa".to_string(), "hwk".to_string()],
                "Executive Leadership",
                "CC-1001-EXEC",
                None,
                true,
                1000usize,
            ),
        };

    let sub = req.sub.clone().unwrap_or_else(|| default_sub.to_string());
    let iss = req
        .issuer
        .clone()
        .unwrap_or_else(|| default_iss.to_string());
    let groups = req.groups.clone().unwrap_or(default_groups);
    let cost_center = req
        .cost_center
        .clone()
        .or_else(|| {
            if default_cc.is_empty() {
                None
            } else {
                Some(default_cc.to_string())
            }
        });

    let claims = OidcClaims {
        sub: sub.clone(),
        iss: iss.clone(),
        aud: DEFAULT_AUDIENCE.to_string(),
        exp,
        iat: now,
        email: Some(format!("{}@jordanlenchitz.com", sub)),
        groups,
        scp: default_scp,
        amr: req.amr.clone().unwrap_or(default_amr),
        department: Some(
            req.department
                .clone()
                .unwrap_or_else(|| default_dept.to_string()),
        ),
        cost_center,
        org_id: req.org_id.clone().or(default_org),
        qpu_entitled: Some(req.qpu_entitled.unwrap_or(default_qpu)),
        max_batch_shots: Some(req.max_batch_shots.unwrap_or(default_batch)),
    };

    let token = mint_jwt(&claims);
    info!(
        "Issued OIDC Bearer token for sub='{}', issuer='{}', persona='{}'",
        claims.sub, claims.iss, persona
    );

    HttpResponse::Ok().json(serde_json::json!({
        "access_token": token,
        "token_type": "Bearer",
        "expires_in": 3600,
        "issuer": claims.iss,
        "claims": claims
    }))
}

#[get("/.well-known/openid-configuration")]
pub async fn openid_configuration() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "issuer": DEFAULT_OKTA_ISSUER,
        "supported_issuers": [
            DEFAULT_OKTA_ISSUER,
            DEFAULT_AUTH0_ISSUER,
            DEFAULT_LOCAL_ISSUER
        ],
        "token_endpoint": "/oauth2/token",
        "jwks_uri": "/.well-known/jwks.json",
        "response_types_supported": ["token", "id_token"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["HS256", "RS256"],
        "scopes_supported": [
            "openid",
            "profile",
            "email",
            "flip:execute",
            "flip:batch",
            "qpu:hardware",
            "scim:manage",
            "governance:review"
        ],
        "claims_supported": [
            "sub",
            "iss",
            "aud",
            "exp",
            "iat",
            "email",
            "groups",
            "scp",
            "amr",
            "department",
            "cost_center",
            "org_id",
            "qpu_entitled",
            "max_batch_shots"
        ]
    }))
}

#[get("/.well-known/jwks.json")]
pub async fn jwks_endpoint() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "keys": [
            {
                "kty": "oct",
                "use": "sig",
                "kid": "jordanlenchitz-iam-key-2026-01",
                "alg": "HS256",
                "issuer_federation": [
                    "Okta Workforce Identity (https://jordanlenchitz.okta.com/oauth2/default)",
                    "Auth0 B2B CIAM (https://identity.jordanlenchitz.com/)"
                ]
            }
        ]
    }))
}
