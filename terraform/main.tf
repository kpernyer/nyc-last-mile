terraform {
  required_version = ">= 1.0"

  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }
  }

  backend "gcs" {
    bucket = "lastmile-terraform-state"
    prefix = "terraform/state"
  }
}

provider "google" {
  project = var.project_id
  region  = var.region
}

# Enable required APIs
resource "google_project_service" "apis" {
  for_each = toset([
    "run.googleapis.com",
    "containerregistry.googleapis.com",
    "cloudbuild.googleapis.com",
    "iam.googleapis.com",
  ])

  service            = each.key
  disable_on_destroy = false
}

# Cloud Run service
resource "google_cloud_run_v2_service" "mcp" {
  name     = var.service_name
  location = var.region

  depends_on = [google_project_service.apis]

  template {
    containers {
      image = "${var.region}-docker.pkg.dev/${var.project_id}/${var.artifact_repo}/${var.service_name}:${var.image_tag}"

      ports {
        container_port = 8080
      }

      resources {
        limits = {
          cpu    = var.cpu
          memory = var.memory
        }
      }

      startup_probe {
        http_get {
          path = "/health"
          port = 8080
        }
        initial_delay_seconds = 10
        period_seconds        = 5
        timeout_seconds       = 5
        failure_threshold     = 10
      }

      liveness_probe {
        http_get {
          path = "/health"
          port = 8080
        }
        period_seconds    = 30
        timeout_seconds   = 5
        failure_threshold = 3
      }

      env {
        name  = "RUST_LOG"
        value = "info"
      }
    }

    scaling {
      min_instance_count = var.min_instances
      max_instance_count = var.max_instances
    }

    timeout = "${var.timeout}s"
  }

  traffic {
    type    = "TRAFFIC_TARGET_ALLOCATION_TYPE_LATEST"
    percent = 100
  }
}

# Allow unauthenticated access
resource "google_cloud_run_v2_service_iam_member" "public" {
  project  = google_cloud_run_v2_service.mcp.project
  location = google_cloud_run_v2_service.mcp.location
  name     = google_cloud_run_v2_service.mcp.name
  role     = "roles/run.invoker"
  member   = "allUsers"
}

# Custom domain mapping
resource "google_cloud_run_domain_mapping" "custom_domain" {
  count    = var.custom_domain != "" ? 1 : 0
  location = var.region
  name     = var.custom_domain

  metadata {
    namespace = var.project_id
  }

  spec {
    route_name = google_cloud_run_v2_service.mcp.name
  }

  depends_on = [google_cloud_run_v2_service.mcp]
}

# Artifact Registry repository (alternative to GCR)
resource "google_artifact_registry_repository" "repo" {
  location      = var.region
  repository_id = var.artifact_repo
  description   = "Docker repository for Last-Mile MCP service"
  format        = "DOCKER"

  depends_on = [google_project_service.apis]
}

# Service account for GitHub Actions
resource "google_service_account" "github_actions" {
  account_id   = "github-actions-deploy"
  display_name = "GitHub Actions Deploy"
  description  = "Service account for GitHub Actions CI/CD"
}

# IAM bindings for the service account
resource "google_project_iam_member" "github_actions_roles" {
  for_each = toset([
    "roles/run.admin",
    "roles/storage.admin",
    "roles/artifactregistry.writer",
    "roles/iam.serviceAccountUser",
  ])

  project = var.project_id
  role    = each.key
  member  = "serviceAccount:${google_service_account.github_actions.email}"
}

# Workload Identity Pool for GitHub OIDC
resource "google_iam_workload_identity_pool" "github" {
  workload_identity_pool_id = "github-pool"
  display_name              = "GitHub Actions Pool"
  description               = "Identity pool for GitHub Actions"
}

resource "google_iam_workload_identity_pool_provider" "github" {
  workload_identity_pool_id          = google_iam_workload_identity_pool.github.workload_identity_pool_id
  workload_identity_pool_provider_id = "github-provider"
  display_name                       = "GitHub Provider"

  attribute_mapping = {
    "google.subject"       = "assertion.sub"
    "attribute.actor"      = "assertion.actor"
    "attribute.repository" = "assertion.repository"
  }

  oidc {
    issuer_uri = "https://token.actions.githubusercontent.com"
  }

  attribute_condition = "assertion.repository == '${var.github_repo}'"
}

# Allow GitHub Actions to impersonate the service account
resource "google_service_account_iam_member" "github_actions_impersonate" {
  service_account_id = google_service_account.github_actions.name
  role               = "roles/iam.workloadIdentityUser"
  member             = "principalSet://iam.googleapis.com/${google_iam_workload_identity_pool.github.name}/attribute.repository/${var.github_repo}"
}
