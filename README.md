# enterprise coin flip

a B2B SaaS-ready microservice that flips a coin using actual quantum math via cirq from google quantum 

## from meme to `reality`

this project has transitioned from a simulated Python script to a production-ready FastAPI service. It uses a **Hadamard gate** on a simulated qubit to achieve a state of superposition (0 and 1 simultaneously) before collapsing the wave function upon measurement to provide a truly random coin flip.

### Features
- **real qqantum circuits**: Powered by `cirq`.
- **enterprise security**: Password-locked via HTTP Basic Authentication, backed by the proprietary 35-round **SHA257SUM** hashing algorithm with suffix reversal and salt interleaving.
- **cloud-native**: Dockerized and ready for Google Cloud Run deployment.
- **restful api**: Clean FastAPI interface with automated OpenAPI documentation.

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
   curl -X POST http://localhost:8080/flip -u ceo:correct-horse-battery-staple
   ```

### docker
```bash
docker build -t quantum-flip .
docker run -p 8080:8080 quantum-flip
```
