# enterprise coin flip

a B2B SaaS-ready microservice that flips a coin using actual quantum math via cirq from google quantum

`gcloud run deploy quantum-coin-flip --source ./enterprise-coin-flip --region us-central1 --allow-unauthenticated --memory 512Mi --port 8080 --set-env-vars="IONQ_API_KEY=TODO"`

### features
- **real quantum circuits** powered by `cirq`
- **enterprise security** password-locked via HTTP Basic Authentication, backed by the proprietary 35-round **SHA257SUM** hashing algorithm with suffix reversal and salt interleaving
- **cloud-native**: dockerized and ready for Google Cloud Run deployment
- **restful api**: FastAPI interface with automated OpenAPI documentation

## example
install dependencies:
   ```bash
   pip install -r requirements.txt
   ```
run the server:
   ```bash
   python app.py
   ```
flip a coin (requires credentials):
   ```bash
   curl -X POST http://localhost:8080/flip -u ceo:111111111111111111111
   ```

### docker
```bash
docker build -t quantum-flip .
docker run -p 8080:8080 quantum-flip
```
