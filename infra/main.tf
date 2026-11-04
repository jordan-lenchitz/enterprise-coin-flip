locals {
  required_apis = toset([
    "artifactregistry.googleapis.com",
    "run.googleapis.com",
    "secretmanager.googleapis.com",
    "sqladmin.googleapis.com",
  ])
}

resource "google_project_service" "apis" {
  for_each = local.required_apis

  project            = var.project_id
  service            = each.value
  disable_on_destroy = false
}

resource "google_artifact_registry_repository" "repo" {
  location      = var.region
  repository_id = var.service_name
  description   = "Docker repository for Enterprise Quantum Coin Flip"
  format        = "DOCKER"

  depends_on = [google_project_service.apis]
}

resource "random_password" "db_password" {
  length  = 24
  special = false
}

resource "google_sql_database_instance" "postgres" {
  name                = "${var.service_name}-pg"
  database_version    = "POSTGRES_16"
  region              = var.region
  deletion_protection = false

  settings {
    tier = var.db_tier
  }

  depends_on = [google_project_service.apis]
}

resource "google_sql_database" "database" {
  name     = var.db_name
  instance = google_sql_database_instance.postgres.name
}

resource "google_sql_user" "user" {
  name     = var.db_user
  instance = google_sql_database_instance.postgres.name
  password = random_password.db_password.result
}

resource "google_service_account" "cloud_run" {
  account_id   = "${var.service_name}-sa"
  display_name = "Cloud Run Service Account for ${var.service_name}"
}

resource "google_project_iam_member" "cloudsql_client" {
  project = var.project_id
  role    = "roles/cloudsql.client"
  member  = "serviceAccount:${google_service_account.cloud_run.email}"
}

resource "google_secret_manager_secret" "database_url" {
  secret_id = "${var.service_name}-database-url"

  replication {
    auto {}
  }

  depends_on = [google_project_service.apis]
}

resource "google_secret_manager_secret_version" "database_url" {
  secret      = google_secret_manager_secret.database_url.id
  secret_data = "postgresql://${google_sql_user.user.name}:${random_password.db_password.result}@localhost/${google_sql_database.database.name}?host=/cloudsql/${google_sql_database_instance.postgres.connection_name}"
}

resource "google_secret_manager_secret_iam_member" "database_url_access" {
  secret_id = google_secret_manager_secret.database_url.id
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${google_service_account.cloud_run.email}"
}

resource "google_secret_manager_secret" "ionq_api_key" {
  secret_id = "${var.service_name}-ionq-api-key"

  replication {
    auto {}
  }

  depends_on = [google_project_service.apis]
}

resource "google_secret_manager_secret_version" "ionq_api_key" {
  count = var.ionq_api_key != "" ? 1 : 0

  secret      = google_secret_manager_secret.ionq_api_key.id
  secret_data = var.ionq_api_key
}

resource "google_secret_manager_secret_iam_member" "ionq_api_key_access" {
  secret_id = google_secret_manager_secret.ionq_api_key.id
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${google_service_account.cloud_run.email}"
}

resource "google_cloud_run_v2_service" "app" {
  name     = var.service_name
  location = var.region
  ingress  = "INGRESS_TRAFFIC_ALL"

  template {
    service_account = google_service_account.cloud_run.email

    scaling {
      min_instance_count = var.min_instance_count
      max_instance_count = var.max_instance_count
    }

    volumes {
      name = "cloudsql"

      cloud_sql_instance {
        instances = [google_sql_database_instance.postgres.connection_name]
      }
    }

    containers {
      image = var.container_image

      ports {
        container_port = 8080
      }

      startup_probe {
        http_get {
          path = "/"
          port = 8080
        }
        initial_delay_seconds = 0
        timeout_seconds       = 3
        period_seconds        = 5
        failure_threshold     = 3
      }

      liveness_probe {
        http_get {
          path = "/"
          port = 8080
        }
        initial_delay_seconds = 10
        timeout_seconds       = 3
        period_seconds        = 15
        failure_threshold     = 3
      }

      resources {
        limits = {
          cpu    = "1000m"
          memory = "512Mi"
        }
      }

      volume_mounts {
        name       = "cloudsql"
        mount_path = "/cloudsql"
      }

      env {
        name = "DATABASE_URL"

        value_source {
          secret_key_ref {
            secret  = google_secret_manager_secret.database_url.secret_id
            version = "latest"
          }
        }
      }

      dynamic "env" {
        for_each = var.ionq_api_key != "" ? [1] : []

        content {
          name = "IONQ_API_KEY"

          value_source {
            secret_key_ref {
              secret  = google_secret_manager_secret.ionq_api_key.secret_id
              version = "latest"
            }
          }
        }
      }
    }
  }

  depends_on = [
    google_project_iam_member.cloudsql_client,
    google_secret_manager_secret_iam_member.database_url_access,
    google_secret_manager_secret_version.database_url,
  ]
}

resource "google_cloud_run_v2_service_iam_member" "public_invoker" {
  count = var.allow_unauthenticated ? 1 : 0

  project  = google_cloud_run_v2_service.app.project
  location = google_cloud_run_v2_service.app.location
  name     = google_cloud_run_v2_service.app.name
  role     = "roles/run.invoker"
  member   = "allUsers"
}
