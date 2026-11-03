variable "project_id" {
  description = "The Google Cloud project ID"
  type        = string
}

variable "region" {
  description = "The Google Cloud region for resources"
  type        = string
  default     = "us-central1"
}

variable "service_name" {
  description = "The name of the Cloud Run service"
  type        = string
  default     = "quantum-coin-flip"
}

variable "container_image" {
  description = "The container image to deploy to Cloud Run"
  type        = string
}

variable "db_tier" {
  description = "The machine tier for the Cloud SQL PostgreSQL instance"
  type        = string
  default     = "db-f1-micro"
}

variable "db_name" {
  description = "The name of the PostgreSQL database"
  type        = string
  default     = "coinflip"
}

variable "db_user" {
  description = "The username for the PostgreSQL database"
  type        = string
  default     = "admin"
}

variable "ionq_api_key" {
  description = "Optional IonQ API key for physical QPU execution"
  type        = string
  default     = ""
  sensitive   = true
}

variable "allow_unauthenticated" {
  description = "Whether to allow unauthenticated public invocations of the Cloud Run service"
  type        = bool
  default     = true
}
