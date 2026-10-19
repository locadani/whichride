output "service_url" {
  description = "The address of the Cloud Run service; open this to check it."
  value       = module.app.service_url
}

output "project_number" {
  description = "GitHub secret GCP_PROJECT_NUMBER."
  value       = data.google_project.this.number
}

output "workload_identity_provider" {
  description = "GitHub secret GCP_WIF_PROVIDER."
  value       = module.app.workload_identity_provider_name
}

output "deploy_service_account" {
  description = "GitHub secret GCP_DEPLOY_SA."
  value       = module.app.deploy_sa_email
}

output "plan_service_account" {
  description = "GitHub secret GCP_PLAN_SA."
  value       = google_service_account.terraform_plan.email
}

output "image_repository" {
  description = "Where deploy.yml pushes images."
  value       = module.app.repository_url
}

output "database_url_secret" {
  description = "Add the Neon connection string as this secret's first version."
  value       = module.app.database_url_secret_id
}

output "backups_bucket" {
  description = "Bucket that holds the nightly database dumps."
  value       = module.app.backups_bucket
}
