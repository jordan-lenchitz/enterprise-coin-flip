# enterprise coin flip

microservice that flips a coin using actual quantum math (via IonQ Aria) or local simulation

`gcloud run deploy quantum-coin-flip --source ./enterprise-coin-flip --region us-central1 --allow-unauthenticated --memory 512Mi --port 8080 --set-env-vars="IONQ_API_KEY=TODO,DATABASE_URL=postgres://user:pass@host:5432/db"`

> **Note**: PostgreSQL is required for this service and the `DATABASE_URL` must be provided and accessible from Cloud Run to avoid crashing on startup

### features
- **real quantum circuits** powered by IonQ iff you want to go 100% quantum
- **enterprise security** backed by federated OIDC (Okta Workforce + Auth0 B2B) or the 35-round [`sha257sum`](https://sha257sum.website) break-glass algorithm
- **scim 2.0 & governance** for workday joiner/mover/leaver provisioning, cost-center chargebacks on $12.42 flips, and quarterly access reviews (see [`docs/IAM_ARCHITECTURE.md`](docs/IAM_ARCHITECTURE.md))
- **cloud-native** and fully ready for google cloud run deployment iff you chose
- **restful api** interface with the usual openAPI docs

## howto
### get started
run the server:
   ```bash
   cargo run
   ```

## example
flip a coin (requires credentials):
   ```bash
   curl -X POST http://localhost:8080/flip -u ceo:111111111111111111111
   ```

or mint an oidc bearer token first if basic auth feels beneath your title:
   ```bash
   TOKEN=$(curl -s -X POST http://localhost:8080/oauth2/token \
     -H "Content-Type: application/json" \
     -d '{"persona": "executive"}' | jq -r .access_token)
   curl -X POST http://localhost:8080/flip -H "Authorization: Bearer $TOKEN"
   ```

### docker
```bash
docker build -t quantum-flip .
docker run -p 8080:8080 quantum-flip
```

### infrastructure (opentofu / terraform)
declarative provisioning (gcp cloud run v2, cloud sql postgresql 16, secret manager, artifact registry, okta, and auth0) lives in [`infra/`](infra/) and works with both **OpenTofu** (`tofu`) and **Terraform** (`terraform`):

```bash
cp infra/terraform.tfvars.example infra/terraform.tfvars
# edit infra/terraform.tfvars with your GCP project_id and container_image

make infra-init
make infra-plan
make infra-apply
```
the service automatically initializes its database schema (`db/init.sql`) on startup.
