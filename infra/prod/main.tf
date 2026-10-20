locals {
  github_repository_id = "1425597390"
}

module "app" {
  source = "../modules/cloud-run-app"

  project_id           = var.project_id
  region               = var.region
  name                 = "whichride"
  github_repository    = "locadani/whichride"
  github_repository_id = local.github_repository_id

  env = {
    RUST_LOG        = "info"
    PLAY_INTEGRITY  = "google"
    ANDROID_PACKAGE = var.android_package
  }

  secret_env = {
    ORS_API_KEY = "ors-api-key"
  }

  # Set to true in a PR once each secret has its first version (infra/README.md).
  attach_database_url = false
  attach_secret_env   = false

  depends_on = [google_project_service.apis]
}
