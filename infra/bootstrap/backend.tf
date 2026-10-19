# First run only: a gitignored backend_override.tf switches this to a local backend, then
# `terraform init -migrate-state -backend-config=backend.hcl` moves the state into the bucket
# this configuration creates. See infra/README.md, first-time setup.
terraform {
  backend "gcs" {
    prefix = "whichride/bootstrap"
  }
}
