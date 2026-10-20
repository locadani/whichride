variable "project_id" {
  description = "Google Cloud project ID (kept out of the repository; see terraform.tfvars.example)."
  type        = string
}

variable "state_bucket" {
  description = "Bucket that holds Terraform state, created by infra/bootstrap."
  type        = string
}

variable "region" {
  description = "Region for every regional resource."
  type        = string
  default     = "europe-west3"
}

variable "android_package" {
  description = "Android application ID whose Play Integrity verdicts the server accepts."
  type        = string
  default     = "io.github.locadani.whichride"
}
