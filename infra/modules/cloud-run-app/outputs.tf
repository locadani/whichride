output "service_name" {
  description = "Cloud Run service name."
  value       = google_cloud_run_v2_service.app.name
}

output "service_url" {
  description = "Default HTTPS address of the service."
  value       = google_cloud_run_v2_service.app.uri
}

output "runtime_sa_email" {
  description = "Account the app runs as."
  value       = google_service_account.runtime.email
}

output "deploy_sa_email" {
  description = "Account the deploy workflow impersonates."
  value       = google_service_account.deploy.email
}

output "repository_url" {
  description = "Where the deploy workflow pushes images."
  value       = "${var.region}-docker.pkg.dev/${var.project_id}/${google_artifact_registry_repository.images.repository_id}"
}

output "database_url_secret_id" {
  description = "Secret that holds the database connection string (value added by hand)."
  value       = google_secret_manager_secret.database_url.secret_id
}

output "workload_identity_pool_name" {
  description = "Full name of the GitHub pool, for further principalSet bindings."
  value       = google_iam_workload_identity_pool.github.name
}

output "workload_identity_provider_name" {
  description = "Full provider name for google-github-actions/auth."
  value       = google_iam_workload_identity_pool_provider.github.name
}

output "backups_bucket" {
  description = "Bucket that holds the nightly database dumps."
  value       = google_storage_bucket.backups.name
}

output "backup_job_name" {
  description = "Cloud Run job that runs the backup; null until attach_database_url is true."
  value       = try(google_cloud_run_v2_job.backup[0].name, null)
}
