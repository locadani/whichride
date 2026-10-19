resource "google_artifact_registry_repository" "images" {
  project       = var.project_id
  location      = var.region
  repository_id = var.name
  format        = "DOCKER"
  description   = "Docker images for ${var.name}"

  cleanup_policy_dry_run = false

  # A keep policy only protects images from delete policies; on its own it deletes nothing.
  cleanup_policies {
    id     = "delete-older-than-1-day"
    action = "DELETE"
    condition {
      tag_state  = "ANY"
      older_than = "1d"
    }
  }

  cleanup_policies {
    id     = "keep-3-most-recent"
    action = "KEEP"
    most_recent_versions {
      keep_count = 3
    }
  }
}

# The value is added by hand: a value managed by Terraform would be stored in its state.
resource "google_secret_manager_secret" "database_url" {
  project             = var.project_id
  secret_id           = "${var.name}-database-url"
  deletion_protection = true

  replication {
    user_managed {
      replicas {
        location = var.region
      }
    }
  }
}

resource "google_service_account" "runtime" {
  project      = var.project_id
  account_id   = "${var.name}-run"
  display_name = "${var.name} on Cloud Run"
}

resource "google_service_account" "deploy" {
  project      = var.project_id
  account_id   = "${var.name}-deploy"
  display_name = "${var.name} deploys from GitHub Actions"
}

resource "google_secret_manager_secret_iam_member" "runtime_reads_database_url" {
  project   = var.project_id
  secret_id = google_secret_manager_secret.database_url.secret_id
  role      = "roles/secretmanager.secretAccessor"
  member    = google_service_account.runtime.member
}

resource "google_artifact_registry_repository_iam_member" "deploy_pushes_images" {
  project    = var.project_id
  location   = var.region
  repository = google_artifact_registry_repository.images.name
  role       = "roles/artifactregistry.writer"
  member     = google_service_account.deploy.member
}

resource "google_service_account_iam_member" "deploy_acts_as_runtime" {
  service_account_id = google_service_account.runtime.name
  role               = "roles/iam.serviceAccountUser"
  member             = google_service_account.deploy.member
}

# Values are added by hand: a value managed by Terraform would be stored in its state.
resource "google_secret_manager_secret" "extra" {
  for_each = var.secret_env

  project             = var.project_id
  secret_id           = "${var.name}-${each.value}"
  deletion_protection = true

  replication {
    user_managed {
      replicas {
        location = var.region
      }
    }
  }
}

resource "google_secret_manager_secret_iam_member" "runtime_reads_extra" {
  for_each = var.secret_env

  project   = var.project_id
  secret_id = google_secret_manager_secret.extra[each.key].secret_id
  role      = "roles/secretmanager.secretAccessor"
  member    = google_service_account.runtime.member
}
