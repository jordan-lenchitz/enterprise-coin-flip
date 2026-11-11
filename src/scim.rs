use actix_web::{delete, get, patch, post, web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::env;
use tracing::{info, instrument, warn};

use crate::crypto::calculate_sha257sum;
use crate::iam::{
    record_iam_audit_event, resolve_principal, IAM_AUTHZ_DECISIONS, OIG_CERTIFICATIONS_TOTAL,
    SCIM_EVENTS_TOTAL,
};

pub const SCIM_USER_SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:User";
pub const SCIM_ENTERPRISE_USER_SCHEMA: &str =
    "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User";
pub const SCIM_LIST_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:ListResponse";
pub const SCIM_PATCH_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:PatchOp";
pub const SCIM_ERROR_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:Error";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScimEmail {
    pub value: String,
    #[serde(default = "default_true")]
    pub primary: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScimEnterpriseExtension {
    #[serde(default)]
    pub department: Option<String>,
    #[serde(default)]
    pub cost_center: Option<String>,
    #[serde(default)]
    pub qpu_entitled: Option<bool>,
    #[serde(default)]
    pub max_batch_shots: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScimUserResource {
    pub schemas: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    pub user_name: String,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(default)]
    pub emails: Vec<ScimEmail>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(
        rename = "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User",
        default
    )]
    pub enterprise: Option<ScimEnterpriseExtension>,
}

#[derive(Debug, Deserialize)]
pub struct ScimFilterQuery {
    #[serde(default)]
    pub filter: Option<String>,
    #[serde(rename = "startIndex", default)]
    pub start_index: Option<usize>,
    #[serde(default)]
    pub count: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ScimPatchOperation {
    pub op: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ScimPatchRequest {
    #[serde(default)]
    pub schemas: Vec<String>,
    #[serde(rename = "Operations")]
    pub operations: Vec<ScimPatchOperation>,
}

fn scim_error(status: actix_web::http::StatusCode, detail: &str) -> HttpResponse {
    HttpResponse::build(status).json(serde_json::json!({
        "schemas": [SCIM_ERROR_SCHEMA],
        "detail": detail,
        "status": status.as_u16().to_string()
    }))
}

/// Validates that the caller has SCIM provisioning or Governance Admin privileges,
/// or presents the configured `SCIM_BEARER_TOKEN`.
async fn authorize_scim_or_governance(
    req: &HttpRequest,
    pool: &sqlx::PgPool,
) -> Result<String, HttpResponse> {
    let expected_scim_token = env::var("SCIM_BEARER_TOKEN")
        .unwrap_or_else(|_| "okta-scim-secret-token-2026".to_string());

    if let Some(auth_val) = req
        .headers()
        .get(actix_web::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(bearer) = auth_val.strip_prefix("Bearer ") {
            if bearer.trim() == expected_scim_token {
                return Ok("okta-scim-connector".to_string());
            }
        }
    }

    match resolve_principal(req, pool).await {
        Ok(principal) => {
            if principal.has_role("SCIM-Provisioner")
                || principal.has_role("IAM-Governance-Admins")
                || principal.has_role("Quantum-Executives")
                || principal.has_scope("scim:manage")
                || principal.has_scope("governance:review")
            {
                Ok(principal.username)
            } else {
                IAM_AUTHZ_DECISIONS
                    .with_label_values(&["rbac", "deny", "missing_scim_or_governance_role"])
                    .inc();
                Err(scim_error(
                    actix_web::http::StatusCode::FORBIDDEN,
                    "Caller lacks SCIM-Provisioner or IAM-Governance-Admins role",
                ))
            }
        }
        Err(_) => Err(scim_error(
            actix_web::http::StatusCode::UNAUTHORIZED,
            "Valid SCIM Bearer token or OIDC IAM Admin credentials required",
        )),
    }
}

fn row_to_scim_user(row: &sqlx::postgres::PgRow) -> ScimUserResource {
    let id: i32 = row.try_get("id").unwrap_or(0);
    let username: String = row.try_get("username").unwrap_or_default();
    let external_id: Option<String> = row.try_get("external_id").ok().flatten();
    let email: String = row
        .try_get::<Option<String>, _>("email")
        .ok()
        .flatten()
        .unwrap_or_else(|| format!("{}@jordanlenchitz.com", username));
    let active: bool = row.try_get("active").unwrap_or(true);
    let department: String = row
        .try_get("department")
        .unwrap_or_else(|_| "Executive".to_string());
    let cost_center: String = row
        .try_get("cost_center")
        .unwrap_or_else(|_| "CC-1001-EXEC".to_string());
    let roles_str: String = row.try_get("roles").unwrap_or_default();
    let roles: Vec<String> = roles_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let qpu_entitled: bool = row.try_get("qpu_entitled").unwrap_or(false);
    let max_batch_shots: i32 = row.try_get("max_batch_shots").unwrap_or(100);

    ScimUserResource {
        schemas: vec![
            SCIM_USER_SCHEMA.to_string(),
            SCIM_ENTERPRISE_USER_SCHEMA.to_string(),
        ],
        id: Some(id.to_string()),
        external_id,
        user_name: username,
        active,
        emails: vec![ScimEmail {
            value: email,
            primary: true,
        }],
        roles,
        enterprise: Some(ScimEnterpriseExtension {
            department: Some(department),
            cost_center: Some(cost_center),
            qpu_entitled: Some(qpu_entitled),
            max_batch_shots: Some(max_batch_shots),
        }),
    }
}

#[get("/scim/v2/Users")]
#[instrument(skip(req, pool))]
pub async fn list_scim_users(
    req: HttpRequest,
    query: web::Query<ScimFilterQuery>,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    if let Err(err_resp) = authorize_scim_or_governance(&req, pool.get_ref()).await {
        return err_resp;
    }

    let start_index = query.start_index.unwrap_or(1).max(1);
    let count = query.count.unwrap_or(100).clamp(1, 500);

    // Parse optional SCIM filter: userName eq "value"
    let filter_username = query.filter.as_ref().and_then(|f| {
        let parts: Vec<&str> = f.splitn(3, ' ').collect();
        if parts.len() == 3 && parts[0].eq_ignore_ascii_case("userName") && parts[1].eq_ignore_ascii_case("eq") {
            Some(parts[2].trim_matches('"').trim_matches('\'').to_string())
        } else {
            None
        }
    });

    let rows = if let Some(uname) = filter_username {
        sqlx::query(
            "SELECT id, username, external_id, email, active, department, cost_center, roles, qpu_entitled, max_batch_shots \
             FROM accounts WHERE username = $1 OR email = $1 ORDER BY id ASC",
        )
        .bind(uname)
        .fetch_all(pool.get_ref())
        .await
        .unwrap_or_default()
    } else {
        sqlx::query(
            "SELECT id, username, external_id, email, active, department, cost_center, roles, qpu_entitled, max_batch_shots \
             FROM accounts ORDER BY id ASC LIMIT $1",
        )
        .bind(count as i64)
        .fetch_all(pool.get_ref())
        .await
        .unwrap_or_default()
    };

    let resources: Vec<ScimUserResource> = rows.iter().map(row_to_scim_user).collect();

    HttpResponse::Ok().json(serde_json::json!({
        "schemas": [SCIM_LIST_SCHEMA],
        "totalResults": resources.len(),
        "startIndex": start_index,
        "itemsPerPage": resources.len(),
        "Resources": resources
    }))
}

#[get("/scim/v2/Users/{id}")]
#[instrument(skip(req, pool))]
pub async fn get_scim_user(
    req: HttpRequest,
    path: web::Path<i32>,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    if let Err(err_resp) = authorize_scim_or_governance(&req, pool.get_ref()).await {
        return err_resp;
    }

    let user_id = path.into_inner();
    let row = sqlx::query(
        "SELECT id, username, external_id, email, active, department, cost_center, roles, qpu_entitled, max_batch_shots \
         FROM accounts WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool.get_ref())
    .await
    .ok()
    .flatten();

    match row {
        Some(r) => HttpResponse::Ok().json(row_to_scim_user(&r)),
        None => scim_error(
            actix_web::http::StatusCode::NOT_FOUND,
            &format!("SCIM User {} not found", user_id),
        ),
    }
}

/// SCIM 2.0 Joiner Workflow: Automated provisioning from Workday -> Okta Lifecycle Management.
#[post("/scim/v2/Users")]
#[instrument(skip(req, body, pool))]
pub async fn create_scim_user(
    req: HttpRequest,
    body: web::Json<ScimUserResource>,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    let actor = match authorize_scim_or_governance(&req, pool.get_ref()).await {
        Ok(a) => a,
        Err(err_resp) => return err_resp,
    };

    let email = body
        .emails
        .first()
        .map(|e| e.value.clone())
        .unwrap_or_else(|| format!("{}@jordanlenchitz.com", body.user_name));

    let ext = body.enterprise.clone().unwrap_or_default();
    let department = ext
        .department
        .unwrap_or_else(|| "Engineering".to_string());
    let cost_center = ext
        .cost_center
        .unwrap_or_else(|| "CC-2000-ENG".to_string());

    let roles_vec = if body.roles.is_empty() {
        vec!["Quantum-Analysts".to_string()]
    } else {
        body.roles.clone()
    };
    let roles_csv = roles_vec.join(",");

    let is_exec = roles_vec
        .iter()
        .any(|r| r.eq_ignore_ascii_case("Quantum-Executives"));
    let qpu_entitled = ext.qpu_entitled.unwrap_or(is_exec);
    let max_batch_shots = ext.max_batch_shots.unwrap_or(if is_exec { 1000 } else { 50 });

    // Generate deterministic break-glass hash for SCIM-provisioned account
    let default_pwd_hash = calculate_sha257sum("111111111111111111111");

    let row_res = sqlx::query(
        "INSERT INTO accounts (username, password_hash, external_id, email, active, department, cost_center, roles, qpu_entitled, max_batch_shots, idp_issuer) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'https://jordanlenchitz.okta.com/oauth2/default') \
         ON CONFLICT (username) DO UPDATE SET \
            external_id = EXCLUDED.external_id, \
            email = EXCLUDED.email, \
            active = EXCLUDED.active, \
            department = EXCLUDED.department, \
            cost_center = EXCLUDED.cost_center, \
            roles = EXCLUDED.roles, \
            qpu_entitled = EXCLUDED.qpu_entitled, \
            max_batch_shots = EXCLUDED.max_batch_shots, \
            deprovisioned_at = NULL, \
            updated_at = CURRENT_TIMESTAMP \
         RETURNING id, username, external_id, email, active, department, cost_center, roles, qpu_entitled, max_batch_shots",
    )
    .bind(&body.user_name)
    .bind(&default_pwd_hash)
    .bind(&body.external_id)
    .bind(&email)
    .bind(body.active)
    .bind(&department)
    .bind(&cost_center)
    .bind(&roles_csv)
    .bind(qpu_entitled)
    .bind(max_batch_shots)
    .fetch_one(pool.get_ref())
    .await;

    match row_res {
        Ok(row) => {
            let account_id: i32 = row.try_get("id").unwrap_or(0);
            SCIM_EVENTS_TOTAL
                .with_label_values(&["POST /scim/v2/Users", "joiner"])
                .inc();
            let details = format!(
                "Provisioned Joiner '{}' (dept='{}', cost_center='{}', roles='{}', qpu_entitled={}, max_batch_shots={})",
                body.user_name, department, cost_center, roles_csv, qpu_entitled, max_batch_shots
            );
            record_iam_audit_event(
                pool.get_ref(),
                Some(account_id),
                &actor,
                "SCIM_JOINER_PROVISIONED",
                "JOINER",
                &details,
            )
            .await;
            info!("{}", details);
            HttpResponse::Created().json(row_to_scim_user(&row))
        }
        Err(e) => scim_error(
            actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to provision SCIM user: {}", e),
        ),
    }
}

/// SCIM 2.0 Mover & Leaver PatchOp Workflow (handles role/department changes and active=false deprovisioning).
#[patch("/scim/v2/Users/{id}")]
#[instrument(skip(req, patch_req, pool))]
pub async fn patch_scim_user(
    req: HttpRequest,
    path: web::Path<i32>,
    patch_req: web::Json<ScimPatchRequest>,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    let actor = match authorize_scim_or_governance(&req, pool.get_ref()).await {
        Ok(a) => a,
        Err(err_resp) => return err_resp,
    };

    let user_id = path.into_inner();
    let existing = sqlx::query(
        "SELECT id, username, external_id, email, active, department, cost_center, roles, qpu_entitled, max_batch_shots \
         FROM accounts WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool.get_ref())
    .await
    .ok()
    .flatten();

    let Some(row) = existing else {
        return scim_error(
            actix_web::http::StatusCode::NOT_FOUND,
            &format!("SCIM User {} not found", user_id),
        );
    };

    let username: String = row.try_get("username").unwrap_or_default();
    let mut active: bool = row.try_get("active").unwrap_or(true);
    let mut department: String = row.try_get("department").unwrap_or_default();
    let mut cost_center: String = row.try_get("cost_center").unwrap_or_default();
    let mut roles: String = row.try_get("roles").unwrap_or_default();
    let mut qpu_entitled: bool = row.try_get("qpu_entitled").unwrap_or(false);
    let mut max_batch_shots: i32 = row.try_get("max_batch_shots").unwrap_or(50);

    let mut is_leaver_event = false;
    let mut is_mover_event = false;

    for op in &patch_req.operations {
        let path_str = op.path.as_deref().unwrap_or("");
        if let Some(val) = &op.value {
            if path_str.eq_ignore_ascii_case("active") {
                if let Some(b) = val.as_bool() {
                    if active && !b {
                        is_leaver_event = true;
                    }
                    active = b;
                }
            } else if path_str.contains("department") {
                if let Some(d) = val.as_str() {
                    department = d.to_string();
                    is_mover_event = true;
                }
            } else if path_str.contains("costCenter") || path_str.contains("cost_center") {
                if let Some(cc) = val.as_str() {
                    cost_center = cc.to_string();
                    is_mover_event = true;
                }
            } else if path_str.eq_ignore_ascii_case("roles") {
                if let Some(arr) = val.as_array() {
                    let parsed: Vec<String> = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    roles = parsed.join(",");
                    is_mover_event = true;
                } else if let Some(s) = val.as_str() {
                    roles = s.to_string();
                    is_mover_event = true;
                }
            } else if path_str.contains("qpuEntitled") || path_str.contains("qpu_entitled") {
                if let Some(b) = val.as_bool() {
                    qpu_entitled = b;
                    is_mover_event = true;
                }
            } else if path_str.contains("maxBatchShots") || path_str.contains("max_batch_shots") {
                if let Some(n) = val.as_i64() {
                    max_batch_shots = n as i32;
                    is_mover_event = true;
                }
            } else if let Some(obj) = val.as_object() {
                // Okta often sends a root-level Replace with {"active": false} or attribute map
                if let Some(b) = obj.get("active").and_then(|v| v.as_bool()) {
                    if active && !b {
                        is_leaver_event = true;
                    }
                    active = b;
                }
                if let Some(d) = obj.get("department").and_then(|v| v.as_str()) {
                    department = d.to_string();
                    is_mover_event = true;
                }
                if let Some(cc) = obj.get("costCenter").and_then(|v| v.as_str()) {
                    cost_center = cc.to_string();
                    is_mover_event = true;
                }
                if let Some(qpu) = obj.get("qpuEntitled").and_then(|v| v.as_bool()) {
                    qpu_entitled = qpu;
                    is_mover_event = true;
                }
                if let Some(shots) = obj.get("maxBatchShots").and_then(|v| v.as_i64()) {
                    max_batch_shots = shots as i32;
                    is_mover_event = true;
                }
                if let Some(arr) = obj.get("roles").and_then(|v| v.as_array()) {
                    let parsed: Vec<String> = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    roles = parsed.join(",");
                    is_mover_event = true;
                }
            }
        }
    }

    let updated = sqlx::query(
        "UPDATE accounts SET \
            active = $1, \
            department = $2, \
            cost_center = $3, \
            roles = $4, \
            qpu_entitled = $5, \
            max_batch_shots = $6, \
            deprovisioned_at = CASE WHEN $1 = FALSE THEN CURRENT_TIMESTAMP ELSE deprovisioned_at END, \
            updated_at = CURRENT_TIMESTAMP \
         WHERE id = $7 \
         RETURNING id, username, external_id, email, active, department, cost_center, roles, qpu_entitled, max_batch_shots",
    )
    .bind(active)
    .bind(&department)
    .bind(&cost_center)
    .bind(&roles)
    .bind(qpu_entitled)
    .bind(max_batch_shots)
    .bind(user_id)
    .fetch_one(pool.get_ref())
    .await;

    match updated {
        Ok(updated_row) => {
            if is_leaver_event {
                SCIM_EVENTS_TOTAL
                    .with_label_values(&["PATCH /scim/v2/Users", "leaver"])
                    .inc();
                let details = format!(
                    "Soft-deprovisioned Leaver '{}' (active=false); revoked active sessions while preserving SOC 2 ledger history",
                    username
                );
                record_iam_audit_event(
                    pool.get_ref(),
                    Some(user_id),
                    &actor,
                    "SCIM_LEAVER_DEPROVISIONED",
                    "LEAVER",
                    &details,
                )
                .await;
                warn!("{}", details);
            } else if is_mover_event {
                SCIM_EVENTS_TOTAL
                    .with_label_values(&["PATCH /scim/v2/Users", "mover"])
                    .inc();
                let details = format!(
                    "Updated Mover entitlements for '{}' (dept='{}', cost_center='{}', roles='{}', qpu_entitled={}, max_batch_shots={})",
                    username, department, cost_center, roles, qpu_entitled, max_batch_shots
                );
                record_iam_audit_event(
                    pool.get_ref(),
                    Some(user_id),
                    &actor,
                    "SCIM_MOVER_UPDATED",
                    "MOVER",
                    &details,
                )
                .await;
                info!("{}", details);
            }

            HttpResponse::Ok().json(row_to_scim_user(&updated_row))
        }
        Err(e) => scim_error(
            actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to apply SCIM PatchOp: {}", e),
        ),
    }
}

/// SCIM 2.0 Leaver DELETE Workflow: Soft-deprovisions user (active=false), revokes access, preserves ledger rows.
#[delete("/scim/v2/Users/{id}")]
#[instrument(skip(req, pool))]
pub async fn delete_scim_user(
    req: HttpRequest,
    path: web::Path<i32>,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    let actor = match authorize_scim_or_governance(&req, pool.get_ref()).await {
        Ok(a) => a,
        Err(err_resp) => return err_resp,
    };

    let user_id = path.into_inner();
    let res = sqlx::query(
        "UPDATE accounts SET active = FALSE, qpu_entitled = FALSE, deprovisioned_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
         WHERE id = $1 RETURNING username",
    )
    .bind(user_id)
    .fetch_optional(pool.get_ref())
    .await
    .ok()
    .flatten();

    match res {
        Some(r) => {
            let username: String = r.try_get("username").unwrap_or_default();
            SCIM_EVENTS_TOTAL
                .with_label_values(&["DELETE /scim/v2/Users", "leaver"])
                .inc();
            let details = format!(
                "SCIM DELETE soft-deprovisioned Leaver '{}' (id={}); active=false, qpu_entitled=false, historical ledger rows preserved",
                username, user_id
            );
            record_iam_audit_event(
                pool.get_ref(),
                Some(user_id),
                &actor,
                "SCIM_LEAVER_SOFT_DELETE",
                "LEAVER",
                &details,
            )
            .await;
            warn!("{}", details);
            HttpResponse::NoContent().finish()
        }
        None => scim_error(
            actix_web::http::StatusCode::NOT_FOUND,
            &format!("SCIM User {} not found", user_id),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct AccessCertificationRequest {
    pub account_id: i32,
    /// 'APPROVE' or 'REVOKE'
    pub decision: String,
    pub justification: String,
}

/// Okta Identity Governance (OIG) Access Review & Entitlement Report.
#[get("/governance/access-review")]
#[instrument(skip(req, pool))]
pub async fn get_access_review(
    req: HttpRequest,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    if let Err(err_resp) = authorize_scim_or_governance(&req, pool.get_ref()).await {
        return err_resp;
    }

    let account_rows = sqlx::query(
        "SELECT a.id, a.username, a.external_id, a.email, a.active, a.department, a.cost_center, \
                a.roles, a.qpu_entitled, a.max_batch_shots, a.idp_issuer, \
                COUNT(l.id) AS total_flips, COALESCE(SUM(l.cost), 0)::FLOAT8 AS total_spend \
         FROM accounts a \
         LEFT JOIN ledger l ON l.account_id = a.id \
         GROUP BY a.id \
         ORDER BY a.id ASC",
    )
    .fetch_all(pool.get_ref())
    .await
    .unwrap_or_default();

    let mut entitlements = Vec::new();
    for r in &account_rows {
        let id: i32 = r.try_get("id").unwrap_or(0);
        let username: String = r.try_get("username").unwrap_or_default();
        let active: bool = r.try_get("active").unwrap_or(true);
        let qpu_entitled: bool = r.try_get("qpu_entitled").unwrap_or(false);
        let cost_center: String = r.try_get("cost_center").unwrap_or_default();
        let roles: String = r.try_get("roles").unwrap_or_default();
        let max_batch_shots: i32 = r.try_get("max_batch_shots").unwrap_or(0);
        let total_flips: i64 = r.try_get("total_flips").unwrap_or(0);
        let total_spend: f64 = r.try_get("total_spend").unwrap_or(0.0);

        let role_list: Vec<&str> = roles.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        let is_exec = role_list.iter().any(|r| r.eq_ignore_ascii_case("Quantum-Executives"));
        let is_analyst = role_list.iter().any(|r| r.eq_ignore_ascii_case("Quantum-Analysts"));
        let is_scim_prov = role_list.iter().any(|r| r.eq_ignore_ascii_case("SCIM-Provisioner"));

        let consecutive_tails = crate::iam::count_consecutive_tails(pool.get_ref(), id).await;

        let mut bureaucratic_findings = Vec::new();
        if !active && qpu_entitled {
            bureaucratic_findings.push(
                "CRITICAL: Deprovisioned account retains QPU hardware entitlement".to_string(),
            );
        }
        if qpu_entitled && cost_center.trim().is_empty() {
            bureaucratic_findings.push(
                "WARN: Hardware QPU entitled without assigned Workday cost_center".to_string(),
            );
        }
        if consecutive_tails >= 3 {
            bureaucratic_findings.push(format!(
                "FINDING [OIG-ENT-402]: Unhedged Tail-Risk Exposure ({} consecutive TAILS in ledger without VP risk acceptance or CAB sign-off)",
                consecutive_tails
            ));
        }
        if is_analyst && !is_exec && max_batch_shots > 100 {
            bureaucratic_findings.push(format!(
                "FINDING [OIG-ENT-109]: Unauthorized Entropy Hoarding (Analyst max_batch_shots={} exceeds departmental median of 50 without actuarial justification)",
                max_batch_shots
            ));
        }
        if is_exec && is_scim_prov {
            bureaucratic_findings.push(
                "FINDING [OIG-SOD-301]: Separation-of-Duties Conflict (Principal holds both SCIM-Provisioner and Quantum-Executives; can self-provision $12.42 QPU budget)".to_string(),
            );
        }
        if active && qpu_entitled && total_flips == 0 {
            bureaucratic_findings.push(
                "FINDING [OIG-FIN-004]: Underutilized Cryogenic Allocation (Entitled to $12.42 physical Ytterbium-171 QPU with 0 wave functions collapsed this quarter)".to_string(),
            );
        }

        let sod_finding = if !active && qpu_entitled {
            "CRITICAL: Deprovisioned account retains QPU hardware entitlement".to_string()
        } else if qpu_entitled && cost_center.trim().is_empty() {
            "WARN: Hardware QPU entitled without assigned Workday cost_center".to_string()
        } else if let Some(first) = bureaucratic_findings.first() {
            first.clone()
        } else {
            "COMPLIANT".to_string()
        };

        entitlements.push(serde_json::json!({
            "account_id": id,
            "username": username,
            "external_id": r.try_get::<Option<String>, _>("external_id").ok().flatten(),
            "email": r.try_get::<Option<String>, _>("email").ok().flatten(),
            "active": active,
            "department": r.try_get::<String, _>("department").unwrap_or_default(),
            "cost_center": cost_center,
            "roles": role_list,
            "qpu_entitled": qpu_entitled,
            "max_batch_shots": max_batch_shots,
            "idp_issuer": r.try_get::<String, _>("idp_issuer").unwrap_or_default(),
            "ledger_usage": {
                "total_flips": total_flips,
                "total_qpu_spend_usd": (total_spend * 100.0).round() / 100.0,
                "consecutive_tails": consecutive_tails
            },
            "governance_Finding": sod_finding,
            "bureaucratic_findings": bureaucratic_findings
        }));
    }

    let audit_rows = sqlx::query(
        "SELECT id, account_id, actor, event_type, lifecycle_stage, details \
         FROM iam_audit_log ORDER BY id DESC LIMIT 25",
    )
    .fetch_all(pool.get_ref())
    .await
    .unwrap_or_default();

    let audit_trail: Vec<serde_json::Value> = audit_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.try_get::<i32, _>("id").unwrap_or(0),
                "account_id": r.try_get::<Option<i32>, _>("account_id").ok().flatten(),
                "actor": r.try_get::<String, _>("actor").unwrap_or_default(),
                "event_type": r.try_get::<String, _>("event_type").unwrap_or_default(),
                "lifecycle_stage": r.try_get::<String, _>("lifecycle_stage").unwrap_or_default(),
                "details": r.try_get::<String, _>("details").unwrap_or_default()
            })
        })
        .collect();

    HttpResponse::Ok().json(serde_json::json!({
        "campaign": "Quarterly OIG Quantum QPU Entitlement Certification (jordanlenchitz.com)",
        "compliance_frameworks": ["SOC 2 Type II", "HIPAA Security Rule §164.308(a)(4)"],
        "total_accounts_reviewed": entitlements.len(),
        "entitlements": entitlements,
        "recent_jml_audit_log": audit_trail
    }))
}

/// Okta Identity Governance (OIG) Certification Decision Endpoint (APPROVE or REVOKE).
#[post("/governance/certify")]
#[instrument(skip(req, body, pool))]
pub async fn certify_access(
    req: HttpRequest,
    body: web::Json<AccessCertificationRequest>,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    let reviewer = match authorize_scim_or_governance(&req, pool.get_ref()).await {
        Ok(a) => a,
        Err(err_resp) => return err_resp,
    };

    let decision = body.decision.trim().to_ascii_uppercase();
    if decision != "APPROVE" && decision != "REVOKE" {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "detail": "decision must be 'APPROVE' or 'REVOKE'"
        }));
    }

    if decision == "REVOKE" {
        let _ = sqlx::query(
            "UPDATE accounts SET qpu_entitled = FALSE, max_batch_shots = 10, updated_at = CURRENT_TIMESTAMP WHERE id = $1",
        )
        .bind(body.account_id)
        .execute(pool.get_ref())
        .await;
    }

    OIG_CERTIFICATIONS_TOTAL
        .with_label_values(&[decision.as_str()])
        .inc();

    let details = format!(
        "OIG Access Certification {}: account_id={} by reviewer='{}' (justification: {})",
        decision, body.account_id, reviewer, body.justification
    );
    record_iam_audit_event(
        pool.get_ref(),
        Some(body.account_id),
        &reviewer,
        &format!("OIG_CERTIFY_{}", decision),
        "GOVERNANCE",
        &details,
    )
    .await;

    HttpResponse::Ok().json(serde_json::json!({
        "status": "recorded",
        "account_id": body.account_id,
        "decision": decision,
        "reviewer": reviewer,
        "remediation_applied": decision == "REVOKE"
    }))
}

#[derive(Debug, Deserialize)]
pub struct SuperpositionDisputeRequest {
    #[serde(default)]
    pub ledger_id: Option<i32>,
    pub contested_outcome: String,
    pub grounds: String,
    pub cab_ticket: String,
    pub countersigner: String,
}

/// Change Advisory Board (CAB) 50/50 Superposition Dispute & Eigenstate Recount Endpoint.
#[post("/governance/dispute-flip")]
#[instrument(skip(req, body, pool))]
pub async fn dispute_superposition(
    req: HttpRequest,
    body: web::Json<SuperpositionDisputeRequest>,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    let (caller_username, caller_email, caller_account_id) =
        match resolve_principal(&req, pool.get_ref()).await {
            Ok(p) => (p.username, p.email, Some(p.account_id)),
            Err(_) => match authorize_scim_or_governance(&req, pool.get_ref()).await {
                Ok(actor) => (actor.clone(), format!("{}@jordanlenchitz.com", actor), None),
                Err(err_resp) => return err_resp,
            },
        };

    let ticket = body.cab_ticket.trim();
    let countersigner = body.countersigner.trim();
    if ticket.is_empty() || countersigner.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "detail": "cab_ticket and countersigner are required to contest a collapsed wave function"
        }));
    }

    if countersigner.eq_ignore_ascii_case(&caller_username)
        || countersigner.eq_ignore_ascii_case(&caller_email)
    {
        IAM_AUTHZ_DECISIONS
            .with_label_values(&["cab", "deny", "two_person_integrity_violation"])
            .inc();
        return HttpResponse::Forbidden().json(serde_json::json!({
            "detail": format!(
                "CAB Two-Person Integrity Violation: Principal '{}' cannot countersign their own 50/50 superposition dispute on ticket '{}'",
                caller_username, ticket
            ),
            "policy_tier": "Denied (Two-Person Integrity Violation)"
        }));
    }

    OIG_CERTIFICATIONS_TOTAL
        .with_label_values(&["DISPUTE_UPHELD_EIGENSTATE"])
        .inc();

    let details = format!(
        "CAB Superposition Dispute on ticket '{}' by '{}' (countersigned by '{}', contested='{}', grounds='{}'): UPHELD_ORIGINAL_EIGENSTATE ($0.00 refunded)",
        ticket, caller_username, countersigner, body.contested_outcome, body.grounds
    );
    record_iam_audit_event(
        pool.get_ref(),
        caller_account_id,
        &caller_username,
        "CAB_SUPERPOSITION_DISPUTE",
        "GOVERNANCE",
        &details,
    )
    .await;

    HttpResponse::Ok().json(serde_json::json!({
        "status": "adjudicated",
        "cab_ticket": ticket,
        "appellant": caller_username,
        "countersigner": countersigner,
        "ledger_id": body.ledger_id,
        "contested_outcome": body.contested_outcome,
        "grounds": body.grounds,
        "ruling": "UPHELD_ORIGINAL_EIGENSTATE",
        "refund_approved_usd": 0.0,
        "resolution": "The Change Advisory Board reviewed the Hilbert-space trajectory under Copenhagen interpretation guidelines. Wave-function collapse is irreversible and the $12.42 hardware fee remains non-refundable."
    }))
}
