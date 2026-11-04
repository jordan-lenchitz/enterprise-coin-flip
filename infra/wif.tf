resource "google_iam_workload_identity_pool" "github" {
  workload_identity_pool_id = "${var.service_name}-gh-pool"
  display_name              = "GitHub Actions Pool"
  description               = "Keyless OIDC Workload Identity Pool for GitHub Actions CI/CD"

  depends_on = [google_project_service.apis]
}

resource "google_iam_workload_identity_pool_provider" "github" {
  workload_identity_pool_id          = google_iam_workload_identity_pool.github.workload_identity_pool_id
  workload_identity_pool_provider_id = "github-oidc"
  display_name                       = "GitHub Actions OIDC Provider"
  description                        = "OIDC provider scoped strictly to ${var.github_repository}"

  attribute_mapping = {
    "google.subject"       = "assertion.sub"
    "attribute.actor"      = "assertion.actor"
    "attribute.repository" = "assertion.repository"
    "attribute.ref"        = "assertion.ref"
  }

  attribute_condition = "assertion.repository == '${var.github_repository}'"

  oidc {
    issuer_uri = "https://token.actions.githubusercontent.com"
  }
}

# 1. Read-Only CI Planner Service Account (used on Pull Requests for `tofu plan`)
resource "google_service_account" "ci_planner" {
  account_id   = "${var.service_name}-planner"
  display_name = "CI Planner SA (Read-Only PR Plans for ${var.service_name})"
}

resource "google_project_iam_member" "ci_planner_viewer" {
  project = var.project_id
  role    = "roles/viewer"
  member  = "serviceAccount:${google_service_account.ci_planner.email}"
}

resource "google_service_account_iam_member" "wif_planner_binding" {
  service_account_id = google_service_account.ci_planner.name
  role               = "roles/iam.workloadIdentityUser"
  member             = "principalSet://iam.googleapis.com/${google_iam_workload_identity_pool.github.name}/attribute.repository/${var.github_repository}"
}

# 2. Least-Privilege CI Deployer Service Account (restricted strictly to refs/heads/main)
resource "google_service_account" "ci_deployer" {
  account_id   = "${var.service_name}-deployer"
  display_name = "CI Deployer SA (Main Branch CD for ${var.service_name})"
}

resource "google_artifact_registry_repository_iam_member" "deployer_repo_writer" {
  project    = google_artifact_registry_repository.repo.project
  location   = google_artifact_registry_repository.repo.location
  repository = google_artifact_registry_repository.repo.name
  role       = "roles/artifactregistry.writer"
  member     = "serviceAccount:${google_service_account.ci_deployer.email}"
}

resource "google_project_iam_member" "deployer_run_developer" {
  project = var.project_id
  role    = "roles/run.developer"
  member  = "serviceAccount:${google_service_account.ci_deployer.email}"
}

resource "google_service_account_iam_member" "deployer_act_as_runtime" {
  service_account_id = google_service_account.cloud_run.name
  role               = "roles/iam.serviceAccountUser"
  member             = "serviceAccount:${google_service_account.ci_deployer.email}"
}

resource "google_service_account_iam_member" "wif_deployer_binding" {
  service_account_id = google_service_account.ci_deployer.name
  role               = "roles/iam.workloadIdentityUser"
  member             = "principal://iam.googleapis.com/${google_iam_workload_identity_pool.github.name}/subject/repo:${var.github_repository}:ref:refs/heads/main"
}
