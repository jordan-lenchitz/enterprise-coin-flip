resource "google_monitoring_uptime_check_config" "healthz" {
  display_name = "${var.service_name}-healthz-uptime"
  timeout      = "10s"
  period       = "60s"

  http_check {
    path           = "/healthz"
    port           = 443
    use_ssl        = true
    validate_ssl   = true
    request_method = "GET"
  }

  monitored_resource {
    type = "uptime_url"
    labels = {
      project_id = var.project_id
      host       = replace(google_cloud_run_v2_service.app.uri, "https://", "")
    }
  }

  depends_on = [google_project_service.apis]
}

resource "google_monitoring_custom_service" "coin_flip" {
  service_id   = "${var.service_name}-slo-svc"
  display_name = "Enterprise Quantum Coin Flip SLO Service"

  depends_on = [google_project_service.apis]
}

resource "google_monitoring_slo" "availability" {
  service             = google_monitoring_custom_service.coin_flip.service_id
  slo_id              = "${var.service_name}-availability-slo"
  display_name        = "99.5% Availability SLO (Rolling 28-Day)"
  goal                = 0.995
  rolling_period_days = 28

  request_based_sli {
    good_total_ratio {
      good_service_filter = join(" AND ", [
        "metric.type=\"run.googleapis.com/request_count\"",
        "resource.type=\"cloud_run_revision\"",
        "resource.label.\"service_name\"=\"${google_cloud_run_v2_service.app.name}\"",
        "metric.label.\"response_code_class\"!=\"5xx\"",
      ])
      total_service_filter = join(" AND ", [
        "metric.type=\"run.googleapis.com/request_count\"",
        "resource.type=\"cloud_run_revision\"",
        "resource.label.\"service_name\"=\"${google_cloud_run_v2_service.app.name}\"",
      ])
    }
  }
}

resource "google_monitoring_slo" "latency" {
  service             = google_monitoring_custom_service.coin_flip.service_id
  slo_id              = "${var.service_name}-latency-slo"
  display_name        = "95% Latency < 500ms SLO (Rolling 28-Day)"
  goal                = 0.95
  rolling_period_days = 28

  request_based_sli {
    distribution_cut {
      distribution_filter = join(" AND ", [
        "metric.type=\"run.googleapis.com/request_latencies\"",
        "resource.type=\"cloud_run_revision\"",
        "resource.label.\"service_name\"=\"${google_cloud_run_v2_service.app.name}\"",
      ])
      range {
        min = 0
        max = 500
      }
    }
  }
}

resource "google_monitoring_alert_policy" "availability_fast_burn" {
  display_name = "${var.service_name} - Fast Error Budget Burn Rate (10x over 1h)"
  combiner     = "OR"

  conditions {
    display_name = "1h Error Budget Burn Rate > 10x"

    condition_threshold {
      filter          = "select_slo_burn_rate(\"${google_monitoring_slo.availability.id}\", \"3600s\")"
      duration        = "0s"
      comparison      = "COMPARISON_GT"
      threshold_value = 10.0

      trigger {
        count = 1
      }
    }
  }
}

resource "google_monitoring_alert_policy" "iam_authz_spike" {
  display_name = "${var.service_name} - IAM OIDC/SCIM 401 & 403 Spike Alert"
  combiner     = "OR"

  conditions {
    display_name = "Elevated 4xx IAM Authentication/Authorization Denials"

    condition_threshold {
      filter = join(" AND ", [
        "metric.type=\"run.googleapis.com/request_count\"",
        "resource.type=\"cloud_run_revision\"",
        "resource.label.\"service_name\"=\"${google_cloud_run_v2_service.app.name}\"",
        "metric.label.\"response_code_class\"=\"4xx\"",
      ])
      duration        = "300s"
      comparison      = "COMPARISON_GT"
      threshold_value = 25.0

      aggregations {
        alignment_period   = "60s"
        per_series_aligner = "ALIGN_RATE"
      }

      trigger {
        count = 1
      }
    }
  }
}
