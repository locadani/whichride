# The bucket name is not committed: pass it with -backend-config=backend.hcl (see backend.hcl.example).
terraform {
  backend "gcs" {
    prefix = "whichride/prod"
  }
}
