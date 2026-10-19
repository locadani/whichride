output "state_bucket" {
  description = "Use as `bucket` in backend.hcl here and in ../prod, and as the GitHub secret TF_STATE_BUCKET."
  value       = google_storage_bucket.state.name
}
