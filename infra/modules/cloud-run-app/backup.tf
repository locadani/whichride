resource "google_storage_bucket" "backups" {
  project                     = var.project_id
  name                        = "${var.project_id}-${var.name}-backups"
  location                    = var.region
  force_destroy               = false
  uniform_bucket_level_access = true
  public_access_prevention    = "enforced"

  # Expired dumps would otherwise stay (and be billed) for 7 more days in soft delete.
  soft_delete_policy {
    retention_duration_seconds = 0
  }

  lifecycle_rule {
    action {
      type = "Delete"
    }
    condition {
      age = var.backup_retention_days
    }
  }
}

resource "google_service_account" "backup" {
  project      = var.project_id
  account_id   = "${var.name}-backup"
  display_name = "${var.name} nightly database backup"
}

resource "google_secret_manager_secret_iam_member" "backup_reads_database_url" {
  project   = var.project_id
  secret_id = google_secret_manager_secret.database_url.secret_id
  role      = "roles/secretmanager.secretAccessor"
  member    = google_service_account.backup.member
}

# Cloud Storage FUSE lists and reads as well as creates objects.
resource "google_storage_bucket_iam_member" "backup_writes_dumps" {
  bucket = google_storage_bucket.backups.name
  role   = "roles/storage.objectUser"
  member = google_service_account.backup.member
}

resource "google_cloud_run_v2_job" "backup" {
  count = var.attach_database_url ? 1 : 0

  project  = var.project_id
  name     = "${var.name}-backup"
  location = var.region

  # A mistake in the job is fixed by applying again, not by unblocking a destroy.
  deletion_protection = false

  template {
    template {
      service_account       = google_service_account.backup.email
      execution_environment = "EXECUTION_ENVIRONMENT_GEN2"
      max_retries           = 1
      timeout               = "600s"

      containers {
        image   = var.backup_image
        command = ["/bin/sh", "-c"]
        args    = ["f=/backups/${var.name}-$(date -u +%Y%m%dT%H%M%SZ).dump; pg_dump \"$DATABASE_URL\" --format=custom --file=\"$f.partial\" && mv \"$f.partial\" \"$f\""]

        env {
          name = "DATABASE_URL"
          value_source {
            secret_key_ref {
              secret  = google_secret_manager_secret.database_url.secret_id
              version = "latest"
            }
          }
        }

        volume_mounts {
          name       = "backups"
          mount_path = "/backups"
        }
      }

      volumes {
        name = "backups"
        gcs {
          bucket    = google_storage_bucket.backups.name
          read_only = false
        }
      }
    }
  }

  depends_on = [
    google_secret_manager_secret_iam_member.backup_reads_database_url,
    google_storage_bucket_iam_member.backup_writes_dumps,
  ]
}

resource "google_cloud_run_v2_job_iam_member" "backup_runs_job" {
  count = var.attach_database_url ? 1 : 0

  project  = var.project_id
  location = var.region
  name     = google_cloud_run_v2_job.backup[0].name
  role     = "roles/run.invoker"
  member   = google_service_account.backup.member
}

resource "google_cloud_scheduler_job" "backup" {
  count = var.attach_database_url ? 1 : 0

  project          = var.project_id
  region           = var.region
  name             = "${var.name}-backup"
  schedule         = var.backup_schedule
  time_zone        = var.backup_time_zone
  attempt_deadline = "320s"

  http_target {
    http_method = "POST"
    uri         = "https://run.googleapis.com/v2/projects/${var.project_id}/locations/${var.region}/jobs/${google_cloud_run_v2_job.backup[0].name}:run"

    oauth_token {
      service_account_email = google_service_account.backup.email
    }
  }
}
