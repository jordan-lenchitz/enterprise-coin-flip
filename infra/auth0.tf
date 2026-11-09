# ============================================================================
# Auth0 Customer & B2B Identity (CIAM) Federation for External Partners
# ============================================================================

resource "auth0_resource_server" "quantum_api" {
  name                                            = "Enterprise Quantum Coin Flip API"
  identifier                                      = "api://quantum-coin-flip"
  signing_alg                                     = "RS256"
  allow_offline_access                            = false
  token_lifetime                                  = 3600
  skip_consent_for_verifiable_first_party_clients = true
  enforce_policies                                = true
  token_dialect                                   = "access_token_authz"
}

resource "auth0_resource_server_scopes" "quantum_api_scopes" {
  resource_server_identifier = auth0_resource_server.quantum_api.identifier

  scopes {
    name        = "flip:execute"
    description = "Execute a single quantum entropy flip"
  }
  scopes {
    name        = "flip:batch"
    description = "Queue batch quantum entropy flips within partner quota"
  }
  scopes {
    name        = "qpu:hardware"
    description = "Authorize physical $12.42 IonQ Aria QPU execution for B2B organization"
  }
}

resource "auth0_client" "b2b_partner_client" {
  name                          = "Enterprise B2B Reinsurance Quantum Client"
  description                   = "Federated B2B CIAM application for external actuarial and reinsurance partners"
  app_type                      = "regular_web"
  oidc_conformant               = true
  callbacks                     = ["${google_cloud_run_v2_service.app.uri}/oauth2/callback"]
  allowed_logout_urls           = [google_cloud_run_v2_service.app.uri]
  grant_types                   = ["authorization_code", "client_credentials", "refresh_token"]
  organization_usage            = "require"
  organization_require_behavior = "pre_login_prompt"
}

resource "auth0_organization" "reinsurance_partner_org" {
  name         = "org-jordanlenchitz-reinsure-01"
  display_name = "Apex Reinsurance Actuarial Partners"

  metadata = {
    cost_center     = "CC-8800-B2B"
    qpu_entitled    = "true"
    max_batch_shots = "250"
    tier            = "b2b_enterprise"
  }
}

resource "auth0_action" "inject_b2b_abac_claims" {
  name    = "inject-b2b-quantum-abac-claims"
  runtime = "node18"
  deploy  = true

  supported_triggers {
    id      = "post-login"
    version = "v3"
  }

  code = <<-EOT
    exports.onExecutePostLogin = async (event, api) => {
      if (!event.organization) {
        api.access.deny("B2B Organization context is required for external CIAM quantum access.");
        return;
      }
      const orgMeta = event.organization.metadata || {};
      api.accessToken.setCustomClaim("org_id", event.organization.id);
      api.accessToken.setCustomClaim("groups", ["CIAM-B2B-Partner"]);
      api.accessToken.setCustomClaim("cost_center", orgMeta.cost_center || "CC-8800-B2B");
      api.accessToken.setCustomClaim("qpu_entitled", orgMeta.qpu_entitled === "true");
      api.accessToken.setCustomClaim("max_batch_shots", parseInt(orgMeta.max_batch_shots || "250", 10));
    };
  EOT
}

resource "auth0_trigger_actions" "post_login_flow" {
  trigger = "post-login"

  actions {
    id           = auth0_action.inject_b2b_abac_claims.id
    display_name = auth0_action.inject_b2b_abac_claims.name
  }
}
