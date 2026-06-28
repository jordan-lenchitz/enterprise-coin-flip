# enterprise coin flip

a B2B SaaS-ready microservice that flips a coin using actual quantum math via IonQ Aria or local simulation.

`gcloud run deploy quantum-coin-flip --source ./enterprise-coin-flip --region us-central1 --allow-unauthenticated --memory 512Mi --port 8080 --set-env-vars="IONQ_API_KEY=TODO,DATABASE_URL=postgres://user:pass@host:5432/db"`

> **Note**: A PostgreSQL database is required for this service. The `DATABASE_URL` must be provided and accessible from Cloud Run, otherwise the application will crash on startup. If using Cloud SQL, you may also need to attach a Cloud SQL connection.
### features
- **real quantum circuits** powered by IonQ iff you want to go 100% quantum
- **enterprise security** backed by the 35-round [`sha257sum`](https://sha257sum.website) algorithm
- **cloud-native** and fully ready for google cloud run deployment iff you chose
- **restful api** interface with the usual openAPI docs

## implementations
### rust
run the server:
   ```bash
   cargo run
   ```

## example
flip a coin (requires credentials):
   ```bash
   curl -X POST http://localhost:8080/flip -u ceo:111111111111111111111
   ```

### docker
```bash
docker build -t quantum-flip .
docker run -p 8080:8080 quantum-flip
```
