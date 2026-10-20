mock_provider "google" {
  mock_data "google_project" {
    defaults = {
      number = "123456789012"
    }
  }
}

variables {
  project_id  = "test-project"
  bucket_name = "test-project-tfstate"

  billing_account_id = "AAAAAA-BBBBBB-CCCCCC"
}

run "state_bucket_is_private_versioned_and_regional" {
  command = plan

  assert {
    condition     = google_storage_bucket.state.location == "EUROPE-WEST3"
    error_message = "The state bucket lives in the app's region."
  }
  assert {
    condition     = google_storage_bucket.state.public_access_prevention == "enforced" && google_storage_bucket.state.uniform_bucket_level_access == true
    error_message = "The state bucket must never be public."
  }
  assert {
    condition     = google_storage_bucket.state.versioning[0].enabled == true
    error_message = "Versioning keeps older state files for recovery."
  }
  assert {
    condition     = google_storage_bucket.state.force_destroy == false
    error_message = "Terraform must not empty the bucket to delete it."
  }
}

run "old_state_versions_are_pruned" {
  command = plan

  assert {
    condition     = anytrue([for r in google_storage_bucket.state.lifecycle_rule : tolist(r.action)[0].type == "Delete" && tolist(r.condition)[0].num_newer_versions == 10 && tolist(r.condition)[0].with_state == "ARCHIVED"])
    error_message = "Only older copies beyond the newest 10 are deleted."
  }
}

run "budget_watches_only_this_project_and_costs_one_euro" {
  command = plan

  assert {
    condition     = tolist(google_billing_budget.monthly.budget_filter)[0].projects == toset(["projects/123456789012"])
    error_message = "The budget covers this project only."
  }
  assert {
    condition     = tolist(tolist(google_billing_budget.monthly.amount)[0].specified_amount)[0].currency_code == "EUR" && tolist(tolist(google_billing_budget.monthly.amount)[0].specified_amount)[0].units == "1"
    error_message = "The budget is 1 EUR a month."
  }
}

run "budget_alerts_at_half_most_and_all_of_it" {
  command = plan

  assert {
    condition     = toset([for t in google_billing_budget.monthly.threshold_rules : t.threshold_percent]) == toset([0.5, 0.9, 1.0])
    error_message = "Alerts at 50%, 90% and 100% of the budget."
  }
  assert {
    condition     = alltrue([for t in google_billing_budget.monthly.threshold_rules : t.spend_basis == "CURRENT_SPEND"])
    error_message = "Alerts are based on the actual spend so far."
  }
}
