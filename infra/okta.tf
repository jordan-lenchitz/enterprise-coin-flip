# ============================================================================
# Okta Workforce Identity, Workday JML Lifecycle, ABAC Claims & OIG Governance
# ============================================================================

resource "okta_group" "quantum_executives" {
  name        = "Quantum-Executives"
  description = "Workforce executives authorized for $12.42 physical IonQ Aria QPU hardware and 1000-shot batches"
}

resource "okta_group" "quantum_analysts" {
  name        = "Quantum-Analysts"
  description = "Actuarial and SRE analysts scoped to local state-vector simulator and 50-shot batches unless ABAC-upgraded"
}

resource "okta_group" "scim_provisioners" {
  name        = "SCIM-Provisioner"
  description = "Automated lifecycle automation service principals permitted to invoke /scim/v2/Users"
}

resource "okta_group" "iam_governance_admins" {
  name        = "IAM-Governance-Admins"
  description = "Okta Identity Governance (OIG) reviewers permitted to run /governance/access-review campaigns"
}

# Custom Okta Universal Directory attributes sourced from Workday HRIS
resource "okta_user_schema_property" "cost_center" {
  index       = "costCenter"
  title       = "Workday Cost Center"
  type        = "string"
  description = "Billing cost center synced from Workday for ABAC hardware QPU chargeback"
  master      = "PROFILE_MASTER"
}

resource "okta_user_schema_property" "qpu_entitled" {
  index       = "qpuEntitled"
  title       = "IonQ Hardware QPU Entitled"
  type        = "boolean"
  description = "ABAC entitlement flag governed by Okta Identity Governance (OIG) access certification"
  master      = "OKTA"
}

resource "okta_user_schema_property" "max_batch_shots" {
  index       = "maxBatchShots"
  title       = "Max Quantum Batch Shots"
  type        = "number"
  description = "Per-identity ABAC batch shot quota enforced by the Rust microservice"
  master      = "OKTA"
}

# Automated Joiner/Mover Group Rules driven by Workday HR attributes
resource "okta_group_rule" "workday_executives_jml" {
  name              = "Workday-JML-Quantum-Executives"
  status            = "ACTIVE"
  group_assignments = [okta_group.quantum_executives.id]
  expression_type   = "urn:okta:expression:1.0"
  expression_value  = "user.department==\"Executive Leadership\" AND String.startsWith(user.costCenter, \"CC-1001\")"
}

resource "okta_group_rule" "workday_analysts_jml" {
  name              = "Workday-JML-Quantum-Analysts"
  status            = "ACTIVE"
  group_assignments = [okta_group.quantum_analysts.id]
  expression_type   = "urn:okta:expression:1.0"
  expression_value  = "user.department==\"Actuarial Science\" OR user.department==\"SRE & IAM Infrastructure\""
}

# OIDC Application for Enterprise Quantum Coin Flip
resource "okta_app_oauth" "quantum_coin_flip" {
  label                      = "Enterprise Quantum Coin Flip (${var.service_name})"
  type                       = "web"
  grant_types                = ["authorization_code", "refresh_token", "client_credentials"]
  redirect_uris              = ["${google_cloud_run_v2_service.app.uri}/oauth2/callback"]
  post_logout_redirect_uris  = [google_cloud_run_v2_service.app.uri]
  response_types             = ["code"]
  token_endpoint_auth_method = "client_secret_basic"
  groups_claim {
    type        = "FILTER"
    filter_type = "REGEX"
    name        = "groups"
    value       = "(Quantum-.*|SCIM-.*|IAM-Governance-.*)"
  }
}

resource "okta_app_group_assignments" "quantum_app_groups" {
  app_id = okta_app_oauth.quantum_coin_flip.id

  group {
    id       = okta_group.quantum_executives.id
    priority = 1
  }
  group {
    id       = okta_group.quantum_analysts.id
    priority = 2
  }
  group {
    id       = okta_group.scim_provisioners.id
    priority = 3
  }
  group {
    id       = okta_group.iam_governance_admins.id
    priority = 4
  }
}

# Custom OAuth 2.0 / OIDC Authorization Server
resource "okta_auth_server" "quantum_authz" {
  name        = "${var.service_name}-authz-server"
  description = "Okta Workforce Authorization Server issuing RBAC + ABAC claims for Quantum Coin Flip"
  audiences   = ["api://quantum-coin-flip"]
  status      = "ACTIVE"
}

resource "okta_auth_server_scope" "flip_execute" {
  auth_server_id   = okta_auth_server.quantum_authz.id
  name             = "flip:execute"
  description      = "Execute single quantum coin flip"
  consent          = "IMPLICIT"
  metadata_publish = "ALL_CLIENTS"
}

resource "okta_auth_server_scope" "flip_batch" {
  auth_server_id   = okta_auth_server.quantum_authz.id
  name             = "flip:batch"
  description      = "Submit asynchronous batch quantum wave-function collapses"
  consent          = "IMPLICIT"
  metadata_publish = "ALL_CLIENTS"
}

resource "okta_auth_server_scope" "qpu_hardware" {
  auth_server_id   = okta_auth_server.quantum_authz.id
  name             = "qpu:hardware"
  description      = "Target physical $12.42 IonQ Aria QPU hardware"
  consent          = "REQUIRED"
  metadata_publish = "ALL_CLIENTS"
}

# Custom OIDC Access Token Claims for Hybrid RBAC + ABAC Evaluation
resource "okta_auth_server_claim" "groups_claim" {
  auth_server_id    = okta_auth_server.quantum_authz.id
  name              = "groups"
  claim_type        = "RESOURCE"
  value_type        = "GROUPS"
  group_filter_type = "REGEX"
  value             = "(Quantum-.*|SCIM-.*|IAM-Governance-.*)"
}

resource "okta_auth_server_claim" "cost_center_claim" {
  auth_server_id = okta_auth_server.quantum_authz.id
  name           = "cost_center"
  claim_type     = "RESOURCE"
  value_type     = "EXPRESSION"
  value          = "user.costCenter"
}

resource "okta_auth_server_claim" "qpu_entitled_claim" {
  auth_server_id = okta_auth_server.quantum_authz.id
  name           = "qpu_entitled"
  claim_type     = "RESOURCE"
  value_type     = "EXPRESSION"
  value          = "user.qpuEntitled"
}

resource "okta_auth_server_claim" "max_batch_shots_claim" {
  auth_server_id = okta_auth_server.quantum_authz.id
  name           = "max_batch_shots"
  claim_type     = "RESOURCE"
  value_type     = "EXPRESSION"
  value          = "user.maxBatchShots"
}
