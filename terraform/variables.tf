variable "project_id" {
  description = "GCP Project ID"
  type        = string
}

variable "region" {
  description = "GCP region for deployment"
  type        = string
  default     = "us-central1"
}

variable "service_name" {
  description = "Name of the Cloud Run service"
  type        = string
  default     = "lastmile-mcp"
}

variable "artifact_repo" {
  description = "Artifact Registry repository name"
  type        = string
  default     = "lastmile"
}

variable "image_tag" {
  description = "Docker image tag"
  type        = string
  default     = "latest"
}

variable "cpu" {
  description = "CPU allocation for Cloud Run"
  type        = string
  default     = "2"
}

variable "memory" {
  description = "Memory allocation for Cloud Run"
  type        = string
  default     = "2Gi"
}

variable "min_instances" {
  description = "Minimum number of instances"
  type        = number
  default     = 0
}

variable "max_instances" {
  description = "Maximum number of instances"
  type        = number
  default     = 10
}

variable "timeout" {
  description = "Request timeout in seconds"
  type        = number
  default     = 300
}

variable "custom_domain" {
  description = "Custom domain for the service (e.g., logistic.hey.sh)"
  type        = string
  default     = ""
}

variable "github_repo" {
  description = "GitHub repository in format owner/repo"
  type        = string
  default     = "kpernyer/nyc-last-mile"
}
