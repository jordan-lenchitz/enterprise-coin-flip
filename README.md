# Enterprise-Grade Quantum Coin Flip

A B2B SaaS-ready microservice that flips a coin using actual Quantum Math via Cirq.

## The Evolution: From Meme to Reality

This project has transitioned from a simulated Python script to a production-ready FastAPI service. It uses a **Hadamard gate** on a simulated qubit to achieve a state of superposition (0 and 1 simultaneously) before collapsing the wave function upon measurement to provide a truly random coin flip.

### Features
- **Real Quantum Circuits**: Powered by `cirq`.
- **Enterprise Security**: Password-locked via HTTP Basic Authentication, backed by the proprietary 35-round **SHA257SUM** hashing algorithm with suffix reversal and salt interleaving.
- **Cloud-Native**: Dockerized and ready for Google Cloud Run deployment.
- **RESTful API**: Clean FastAPI interface with automated OpenAPI documentation.

## Quick Start

### Local Development
1. Install dependencies:
   ```bash
   pip install -r requirements.txt
   ```
2. Run the server:
   ```bash
   python app.py
   ```
3. Flip a coin (requires credentials):
   ```bash
   curl -X POST http://localhost:8080/flip -u ceo:correct-horse-battery-staple
   ```

### Docker
```bash
docker build -t quantum-flip .
docker run -p 8080:8080 quantum-flip
```

## LinkedIn Pitch (Updated)
"I took the meme and made it unironically real. My new enterprise-grade microservice uses actual quantum gate operations to resolve business decisions. It's password-protected because high-level entropy shouldn't be free. #QuantumComputing #B2B #SaaS #EngineeringExcellence"
