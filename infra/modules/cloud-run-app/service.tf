resource "google_cloud_run_v2_service" "app" {
  project              = var.project_id
  name                 = var.name
  location             = var.region
  ingress              = "INGRESS_TRAFFIC_ALL"
  invoker_iam_disabled = true
  deletion_protection  = true

  template {
    service_account = google_service_account.runtime.email

    scaling {
      min_instance_count = 0
      max_instance_count = var.max_instances
    }

    containers {
      image = var.initial_image

      resources {
        limits = {
          cpu    = "1"
          memory = var.memory
        }
        cpu_idle = true
      }

      dynamic "env" {
        for_each = var.env
        iterator = setting
        content {
          name  = setting.key
          value = setting.value
        }
      }

      dynamic "env" {
        for_each = var.attach_database_url ? [google_secret_manager_secret.database_url.secret_id] : []
        content {
          name = "DATABASE_URL"
          value_source {
            secret_key_ref {
              secret  = env.value
              version = "latest"
            }
          }
        }
      }

      dynamic "env" {
        for_each = { for name, secret in var.secret_env : name => secret if contains(var.attached_secrets, name) }
        content {
          name = env.key
          value_source {
            secret_key_ref {
              secret  = google_secret_manager_secret.extra[env.key].secret_id
              version = "latest"
            }
          }
        }
      }
    }
  }

  lifecycle {
    # The deploy workflow owns the running image; without this, every apply would roll it back.
    # gcloud stamps client annotations (e.g. client.knative.dev/user-image) on the revision
    # template, so template labels and annotations are not managed by Terraform.
    ignore_changes = [
      template[0].containers[0].image,
      client,
      client_version,
      template[0].labels,
      template[0].annotations,
    ]
  }

  depends_on = [
    google_secret_manager_secret_iam_member.runtime_reads_database_url,
    google_secret_manager_secret_iam_member.runtime_reads_extra,
  ]
}

resource "google_cloud_run_v2_service_iam_member" "deploy_updates_service" {
  project  = var.project_id
  location = var.region
  name     = google_cloud_run_v2_service.app.name
  role     = "roles/run.developer"
  member   = google_service_account.deploy.member
}
