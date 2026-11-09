output "service_url" {
  description = "The public URL of the deployed Cloud Run service"
  value       = google_cloud_run_v2_service.app.uri
}

output "artifact_registry_repository" {
  description = "The Artifact Registry repository path for pushing container images"
  value       = "${var.region}-docker.pkg.dev/${var.project_id}/${google_artifact_registry_repository.repo.repository_id}"
}

output "cloud_sql_connection_name" {
  description = "The connection name of the Cloud SQL PostgreSQL instance"
  value       = google_sql_database_instance.postgres.connection_name
}

output "workload_identity_provider" {
  description = "The full resource name of the GitHub OIDC Workload Identity Pool Provider"
  value       = google_iam_workload_identity_pool_provider.github.name
}

output "ci_planner_service_account" {
  description = "The email of the read-only CI Planner Service Account for PR plans"
  value       = google_service_account.ci_planner.email
}

output "ci_deployer_service_account" {
  description = "The email of the CI Deployer Service Account for main-branch deployments"
  value       = google_service_account.ci_deployer.email
}

output "okta_oauth_client_id" {
  description = "The Okta Workforce OIDC Client ID for Enterprise Quantum Coin Flip"
  value       = okta_app_oauth.quantum_coin_flip.client_id
}

output "okta_auth_server_issuer" {
  description = "The Okta Workforce Authorization Server Issuer URI"
  value       = okta_auth_server.quantum_authz.issuer
}

output "auth0_b2b_client_id" {
  description = "The Auth0 B2B CIAM Client ID for external reinsurance partners"
  value       = auth0_client.b2b_partner_client.client_id
}
