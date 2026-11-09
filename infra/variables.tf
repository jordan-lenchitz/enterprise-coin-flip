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

variable "min_instance_count" {
  description = "Minimum number of Cloud Run container instances"
  type        = number
  default     = 0
}

variable "max_instance_count" {
  description = "Maximum number of Cloud Run container instances"
  type        = number
  default     = 5
}

variable "github_repository" {
  description = "GitHub repository (owner/repo) allowed to authenticate via Workload Identity Federation"
  type        = string
  default     = "jordan-lenchitz/enterprise-coin-flip"
}

variable "okta_org_name" {
  description = "Okta Workforce Identity organization subdomain (e.g. 'jordanlenchitz')"
  type        = string
  default     = "jordanlenchitz"
}

variable "okta_base_url" {
  description = "Okta base domain (okta.com, oktapreview.com, or okta-emea.com)"
  type        = string
  default     = "okta.com"
}

variable "okta_api_token" {
  description = "Okta SSWS API token or OAuth 2.0 private key token for Terraform IAM management"
  type        = string
  default     = "placeholder-ssws-token"
  sensitive   = true
}

variable "auth0_domain" {
  description = "Auth0 Customer Identity (CIAM) tenant domain for B2B partner federation"
  type        = string
  default     = "identity.jordanlenchitz.com"
}

variable "auth0_client_id" {
  description = "Auth0 Machine-to-Machine Terraform management client ID"
  type        = string
  default     = "placeholder-auth0-client-id"
}

variable "auth0_client_secret" {
  description = "Auth0 Machine-to-Machine Terraform management client secret"
  type        = string
  default     = "placeholder-auth0-client-secret"
  sensitive   = true
}
