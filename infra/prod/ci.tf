# Read-only account for `terraform plan` in pull requests. Applying is done by hand.
resource "google_service_account" "terraform_plan" {
  account_id   = "terraform-plan"
  display_name = "Terraform plan (read-only, GitHub Actions)"

  depends_on = [google_project_service.apis]
}

resource "google_project_iam_member" "terraform_plan" {
  for_each = toset(["roles/viewer", "roles/iam.securityReviewer"])

  project = var.project_id
  role    = each.value
  member  = google_service_account.terraform_plan.member
}

resource "google_storage_bucket_iam_member" "terraform_plan_reads_state" {
  bucket = var.state_bucket
  role   = "roles/storage.objectViewer"
  member = google_service_account.terraform_plan.member
}

# Any workflow of this repository: a binding matches one exact value, and this account is read-only.
resource "google_service_account_iam_member" "terraform_plan_github" {
  service_account_id = google_service_account.terraform_plan.name
  role               = "roles/iam.workloadIdentityUser"
  member             = "principalSet://iam.googleapis.com/${module.app.workload_identity_pool_name}/attribute.repository_id/${local.github_repository_id}"
}
