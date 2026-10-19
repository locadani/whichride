mock_provider "google" {}

# Computed attributes are random strings by default and the IAM resources validate their format;
# each account gets its own identity so a swapped grant cannot pass the equality assertions.
override_resource {
  target = google_service_account.runtime
  values = {
    name   = "projects/test-project/serviceAccounts/whichride-runtime@test-project.iam.gserviceaccount.com"
    email  = "whichride-runtime@test-project.iam.gserviceaccount.com"
    member = "serviceAccount:whichride-runtime@test-project.iam.gserviceaccount.com"
  }
}

override_resource {
  target = google_service_account.deploy
  values = {
    name   = "projects/test-project/serviceAccounts/whichride-deploy@test-project.iam.gserviceaccount.com"
    email  = "whichride-deploy@test-project.iam.gserviceaccount.com"
    member = "serviceAccount:whichride-deploy@test-project.iam.gserviceaccount.com"
  }
}

override_resource {
  target = google_service_account.backup
  values = {
    name   = "projects/test-project/serviceAccounts/whichride-backup@test-project.iam.gserviceaccount.com"
    email  = "whichride-backup@test-project.iam.gserviceaccount.com"
    member = "serviceAccount:whichride-backup@test-project.iam.gserviceaccount.com"
  }
}

variables {
  project_id           = "test-project"
  region               = "europe-west3"
  name                 = "whichride"
  github_repository    = "locadani/whichride"
  github_repository_id = "1425597390"
}

run "registry_deletes_old_images_but_keeps_three" {
  assert {
    condition     = google_artifact_registry_repository.images.format == "DOCKER" && google_artifact_registry_repository.images.location == "europe-west3"
    error_message = "The registry must be a Docker repository in the app's region."
  }
  assert {
    condition     = google_artifact_registry_repository.images.cleanup_policy_dry_run == false
    error_message = "Cleanup must actually delete (dry run off)."
  }
  assert {
    condition     = anytrue([for p in google_artifact_registry_repository.images.cleanup_policies : p.action == "DELETE" && try(p.condition[0].older_than, "") == "1d"])
    error_message = "A delete policy is required: a keep policy alone deletes nothing."
  }
  assert {
    condition     = anytrue([for p in google_artifact_registry_repository.images.cleanup_policies : p.action == "KEEP" && try(p.most_recent_versions[0].keep_count, 0) == 3])
    error_message = "The 3 most recent images must be kept for rollbacks."
  }
}

run "secret_is_regional_protected_and_has_no_value" {
  assert {
    condition     = google_secret_manager_secret.database_url.secret_id == "whichride-database-url"
    error_message = "The secret is named after the app."
  }
  assert {
    condition     = google_secret_manager_secret.database_url.replication[0].user_managed[0].replicas[0].location == "europe-west3"
    error_message = "The secret must be stored only in the app's region."
  }
  assert {
    condition     = google_secret_manager_secret.database_url.deletion_protection == true
    error_message = "The secret must be protected against deletion."
  }
}

run "name_must_fit_account_ids" {
  command = plan

  variables {
    name = "a-very-long-application-name"
  }

  expect_failures = [var.name]
}

run "name_of_23_characters_is_accepted" {
  command = plan

  variables {
    name = "abcdefghij-klmnopqrstuv"
  }
}

run "name_of_5_characters_is_accepted" {
  command = plan

  variables {
    name = "abcde"
  }
}

run "name_ending_in_hyphen_is_rejected" {
  command = plan

  variables {
    name = "whichride-"
  }

  expect_failures = [var.name]
}

run "name_of_4_characters_is_rejected" {
  command = plan

  variables {
    name = "abcd"
  }

  expect_failures = [var.name]
}

run "runtime_account_can_only_read_its_secret" {
  assert {
    condition     = google_service_account.runtime.account_id == "whichride-run"
    error_message = "The runtime account is named <name>-run."
  }
  assert {
    condition     = google_secret_manager_secret_iam_member.runtime_reads_database_url.role == "roles/secretmanager.secretAccessor" && google_secret_manager_secret_iam_member.runtime_reads_database_url.member == google_service_account.runtime.member
    error_message = "The runtime account must be able to read the database secret."
  }
  assert {
    condition     = google_secret_manager_secret_iam_member.runtime_reads_database_url.secret_id == google_secret_manager_secret.database_url.secret_id
    error_message = "The grant must be on the one secret, not the project."
  }
}

run "deploy_account_can_push_images_and_act_as_runtime" {
  assert {
    condition     = google_service_account.deploy.account_id == "whichride-deploy"
    error_message = "The deploy account is named <name>-deploy."
  }
  assert {
    condition     = google_artifact_registry_repository_iam_member.deploy_pushes_images.role == "roles/artifactregistry.writer" && google_artifact_registry_repository_iam_member.deploy_pushes_images.member == google_service_account.deploy.member
    error_message = "The deploy account must be able to push to the app's registry only."
  }
  assert {
    condition     = google_service_account_iam_member.deploy_acts_as_runtime.role == "roles/iam.serviceAccountUser" && google_service_account_iam_member.deploy_acts_as_runtime.service_account_id == google_service_account.runtime.name
    error_message = "The deploy account must be able to run revisions as the runtime account."
  }
}

run "service_is_public_capped_and_protected" {
  assert {
    condition     = google_cloud_run_v2_service.app.invoker_iam_disabled == true
    error_message = "The service is public through invoker_iam_disabled, not an allUsers grant."
  }
  assert {
    condition     = google_cloud_run_v2_service.app.template[0].scaling[0].min_instance_count == 0 && google_cloud_run_v2_service.app.template[0].scaling[0].max_instance_count == 1
    error_message = "The service scales to zero and to at most one instance."
  }
  assert {
    condition     = google_cloud_run_v2_service.app.template[0].containers[0].resources[0].cpu_idle == true
    error_message = "CPU must be allocated only during requests (free tier)."
  }
  assert {
    condition     = google_cloud_run_v2_service.app.template[0].containers[0].resources[0].limits["memory"] == "512Mi"
    error_message = "Memory defaults to 512Mi."
  }
  assert {
    condition     = google_cloud_run_v2_service.app.template[0].service_account == google_service_account.runtime.email
    error_message = "The service runs as the runtime account."
  }
  assert {
    condition     = google_cloud_run_v2_service.app.deletion_protection == true
    error_message = "The service must be protected against deletion."
  }
  assert {
    condition     = google_cloud_run_v2_service_iam_member.deploy_updates_service.role == "roles/run.developer" && google_cloud_run_v2_service_iam_member.deploy_updates_service.member == google_service_account.deploy.member
    error_message = "The deploy account may deploy this one service, and nothing more."
  }
}

run "database_url_not_attached_by_default" {
  assert {
    condition     = length(google_cloud_run_v2_service.app.template[0].containers[0].env) == 0
    error_message = "Before the secret has a value, the service must not reference it, or instances won't start."
  }
}

run "database_url_attached_from_secret_when_enabled" {
  variables {
    attach_database_url = true
  }

  assert {
    condition     = tolist(google_cloud_run_v2_service.app.template[0].containers[0].env)[0].name == "DATABASE_URL"
    error_message = "The app reads its connection string as DATABASE_URL."
  }
  assert {
    condition     = tolist(google_cloud_run_v2_service.app.template[0].containers[0].env)[0].value_source[0].secret_key_ref[0].secret == google_secret_manager_secret.database_url.secret_id && tolist(google_cloud_run_v2_service.app.template[0].containers[0].env)[0].value_source[0].secret_key_ref[0].version == "latest"
    error_message = "DATABASE_URL comes from the latest version of the app's secret."
  }
}

run "only_this_repository_can_log_in" {
  assert {
    condition     = google_iam_workload_identity_pool_provider.github.attribute_condition == "assertion.repository_id == '1425597390'"
    error_message = "Only tokens from this repository (by ID, not name) may be exchanged."
  }
  assert {
    condition     = google_iam_workload_identity_pool_provider.github.oidc[0].issuer_uri == "https://token.actions.githubusercontent.com"
    error_message = "The provider trusts GitHub's OIDC issuer."
  }
  assert {
    condition     = google_iam_workload_identity_pool_provider.github.attribute_mapping["attribute.repository_id"] == "assertion.repository_id" && google_iam_workload_identity_pool_provider.github.attribute_mapping["attribute.job_workflow_ref"] == "assertion.job_workflow_ref"
    error_message = "repository_id and job_workflow_ref must be mapped so bindings can use them."
  }
}

run "only_the_deploy_workflow_on_main_can_deploy" {
  assert {
    condition     = google_service_account_iam_member.github_deploys.role == "roles/iam.workloadIdentityUser" && google_service_account_iam_member.github_deploys.service_account_id == google_service_account.deploy.name
    error_message = "GitHub may impersonate the deploy account only."
  }
  assert {
    condition     = endswith(google_service_account_iam_member.github_deploys.member, "/attribute.job_workflow_ref/locadani/whichride/.github/workflows/deploy.yml@refs/heads/main")
    error_message = "Only deploy.yml running on main may impersonate the deploy account."
  }
  assert {
    condition     = startswith(google_service_account_iam_member.github_deploys.member, "principalSet://iam.googleapis.com/")
    error_message = "The binding is a principalSet on the pool."
  }
}

run "outputs_for_github_settings" {
  assert {
    condition     = output.repository_url == "europe-west3-docker.pkg.dev/test-project/whichride"
    error_message = "The image repository URL is <region>-docker.pkg.dev/<project>/<name>."
  }
  assert {
    condition     = output.database_url_secret_id == "whichride-database-url"
    error_message = "The secret ID is exposed for the setup instructions."
  }
}

run "plain_settings_are_passed_to_the_container" {
  variables {
    env = {
      RUST_LOG         = "info"
      CLIENT_IP_HEADER = "X-Forwarded-For"
    }
  }

  assert {
    condition     = anytrue([for e in google_cloud_run_v2_service.app.template[0].containers[0].env : e.name == "CLIENT_IP_HEADER" && e.value == "X-Forwarded-For"])
    error_message = "Plain settings reach the container."
  }
  assert {
    condition     = length(google_cloud_run_v2_service.app.template[0].containers[0].env) == 2
    error_message = "Without attach_database_url, only the plain settings are set."
  }
}

run "settings_and_database_url_together" {
  variables {
    attach_database_url = true
    env                 = { RUST_LOG = "info" }
  }

  assert {
    condition     = length(google_cloud_run_v2_service.app.template[0].containers[0].env) == 2
    error_message = "One plain setting plus DATABASE_URL."
  }
}

run "backup_bucket_is_private_and_expires_old_dumps" {
  command = plan
  assert {
    condition     = google_storage_bucket.backups.name == "test-project-whichride-backups" && google_storage_bucket.backups.public_access_prevention == "enforced"
    error_message = "Private backups bucket named after project and app."
  }
  assert {
    condition     = anytrue([for r in google_storage_bucket.backups.lifecycle_rule : tolist(r.action)[0].type == "Delete" && tolist(r.condition)[0].age == 30])
    error_message = "Dumps older than 30 days are deleted."
  }
  assert {
    condition     = tolist(google_storage_bucket.backups.soft_delete_policy)[0].retention_duration_seconds == 0
    error_message = "Deleted dumps are not kept (and billed) in soft delete."
  }
}

run "no_backup_job_before_the_secret_has_a_value" {
  command = plan
  assert {
    condition     = length(google_cloud_run_v2_job.backup) == 0 && length(google_cloud_scheduler_job.backup) == 0
    error_message = "The job needs DATABASE_URL, which exists only once attach_database_url is true."
  }
}

run "nightly_backup_when_attached" {
  command = plan
  variables {
    attach_database_url = true
  }
  assert {
    condition     = google_cloud_scheduler_job.backup[0].schedule == "0 3 * * *" && google_cloud_scheduler_job.backup[0].time_zone == "Europe/Berlin"
    error_message = "Runs daily at 03:00 Berlin time."
  }
  assert {
    condition     = google_cloud_run_v2_job.backup[0].template[0].template[0].service_account == google_service_account.backup.email
    error_message = "The job runs as the backup account."
  }
  assert {
    condition     = google_cloud_run_v2_job.backup[0].template[0].template[0].containers[0].args[0] == "f=/backups/whichride-$(date -u +%Y%m%dT%H%M%SZ).dump; pg_dump \"$DATABASE_URL\" --format=custom --file=\"$f.partial\" && mv \"$f.partial\" \"$f\""
    error_message = "The shell expansions must reach the container untouched."
  }
  assert {
    condition     = tolist(google_cloud_scheduler_job.backup[0].http_target)[0].uri == "https://run.googleapis.com/v2/projects/test-project/locations/europe-west3/jobs/whichride-backup:run"
    error_message = "The scheduler calls the job's :run endpoint."
  }
  assert {
    condition     = tolist(tolist(google_cloud_scheduler_job.backup[0].http_target)[0].oauth_token)[0].service_account_email == google_service_account.backup.email
    error_message = "The scheduler calls the job as the backup account."
  }
  assert {
    condition     = google_cloud_run_v2_job_iam_member.backup_runs_job[0].role == "roles/run.invoker" && google_cloud_run_v2_job_iam_member.backup_runs_job[0].member == google_service_account.backup.member
    error_message = "The backup account may run the job, and nothing broader."
  }
}

run "backup_account_grants_are_narrow" {
  command = plan
  assert {
    condition     = google_secret_manager_secret_iam_member.backup_reads_database_url.secret_id == google_secret_manager_secret.database_url.secret_id && google_storage_bucket_iam_member.backup_writes_dumps.bucket == google_storage_bucket.backups.name
    error_message = "The backup account reads one secret and writes one bucket."
  }
  assert {
    condition     = google_secret_manager_secret_iam_member.backup_reads_database_url.role == "roles/secretmanager.secretAccessor" && google_storage_bucket_iam_member.backup_writes_dumps.role == "roles/storage.objectUser"
    error_message = "The backup account holds only the secret accessor and bucket object roles."
  }
}

run "secret_env_creates_secrets_without_attaching_them" {
  command = plan

  variables {
    secret_env = { ORS_API_KEY = "ors-api-key" }
  }

  assert {
    condition     = google_secret_manager_secret.extra["ORS_API_KEY"].secret_id == "${var.name}-ors-api-key"
    error_message = "the secret is named <name>-<suffix>"
  }

  assert {
    condition     = length([for e in google_cloud_run_v2_service.app.template[0].containers[0].env : e if e.name == "ORS_API_KEY"]) == 0
    error_message = "secrets are attached only with attach_secret_env"
  }
}

run "attached_secret_env_reaches_the_container" {
  command = plan

  variables {
    secret_env        = { ORS_API_KEY = "ors-api-key" }
    attach_secret_env = true
  }

  assert {
    condition     = length([for e in google_cloud_run_v2_service.app.template[0].containers[0].env : e if e.name == "ORS_API_KEY"]) == 1
    error_message = "ORS_API_KEY comes from its secret"
  }
}
