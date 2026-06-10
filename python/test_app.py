import os
import pytest
from fastapi.testclient import TestClient
from app import app, calculate_sha257sum, run_quantum_flip

client = TestClient(app)

def test_sha257sum_parity():
    # Known test vector
    expected = "18bb824a4ad1f39be49cc91af302dad50e27f9af7ff17b5dade977dc3beb0a58"
    result = calculate_sha257sum("111111111111111111111")
    assert result == expected

def test_run_quantum_flip_fallback():
    # Ensure IONQ_API_KEY is not set for fallback testing
    if "IONQ_API_KEY" in os.environ:
        del os.environ["IONQ_API_KEY"]
    
    bit, environment = run_quantum_flip()
    assert bit in (0, 1)
    assert environment == "Production-Simulation (Free)"

def test_get_ui():
    response = client.get("/")
    assert response.status_code == 200
    assert "QUANTUM ENTROPY" in response.text
    assert "SIMULATOR ACTIVE: NO API KEY DETECTED" in response.text

def test_flip_unauthorized():
    response = client.post("/flip")
    assert response.status_code == 401

def test_flip_incorrect_credentials():
    response = client.post("/flip", auth=("ceo", "wrongpassword"))
    assert response.status_code == 401

def test_flip_success():
    if "IONQ_API_KEY" in os.environ:
        del os.environ["IONQ_API_KEY"]
    
    response = client.post("/flip", auth=("ceo", "111111111111111111111"))
    assert response.status_code == 200
    data = response.json()
    assert data["status"] == "success"
    assert data["result"] in ("HEADS", "TAILS")
    assert data["quantum_bit"] in (0, 1)
    assert data["metadata"]["environment"] == "Production-Simulation (Free)"
