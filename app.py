import cirq
import os
import logging
import hashlib
from fastapi import FastAPI, Depends, HTTPException, status
from fastapi.security import HTTPBasic, HTTPBasicCredentials
import secrets

# Setup logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger("enterprise-coin-flip")

app = FastAPI(
    title="Enterprise Quantum Coin Flip Service",
    description="Secure, authenticated B2B solution for true quantum bit collapse.",
    version="1.0.0"
)

security = HTTPBasic()

# In a real enterprise app, these would be in Secret Manager
ENTERPRISE_USER = os.getenv("FLIP_USER", "ceo")

# --- Custom SHA257 implementation translated from Jordan's TS ---
STUPID_SALTS = [
    b"jordanlenchitz_absurd_salt_part1_stupid_stupid_stupid_1_LLOC_INCREASE_AA",
    b"jordanlenchitz_absurd_salt_part2_very_silly_nonsense_2_LLOC_ENHANCE_BB",
    b"jordanlenchitz_absurd_salt_part3_utterly_pointless_3_LLOC_MAXIMUM_CC",
    b"jordanlenchitz_absurd_salt_part4_final_silly_bits_4_LLOC_OVER_1000_DD",
    b"jordanlenchitz_absurd_salt_part5_more_random_bytes_5_LLOC_ABUNDANCE_EE",
    b"jordanlenchitz_absurd_salt_part6_extra_long_salt_6_LLOC_GENERATE_FF",
    b"jordanlenchitz_absurd_salt_part7_another_salt_block_7_LLOC_FILL_GG",
    b"jordanlenchitz_absurd_salt_part8_just_for_lines_8_LLOC_MANY_MANY_HH",
    b"jordanlenchitz_absurd_salt_part9_yet_another_salt_9_LLOC_MORE_II",
    b"jordanlenchitz_absurd_salt_part10_final_long_salt_10_LLOC_END_OF_SALTS_JJ"
]

def calculate_sha257sum(data: str) -> str:
    def _sha256(msg: bytes) -> str:
        return hashlib.sha256(msg).hexdigest()

    current = data.encode('utf-8')

    for i in range(35):
        hash_hex = _sha256(current)
        prefix = hash_hex[:-8]
        suffix = hash_hex[-8:]
        reversed_suffix = suffix[::-1]
        intermediate_hex = prefix + reversed_suffix
        intermediate_bytes = intermediate_hex.encode('utf-8')

        salt = STUPID_SALTS[i % 10]
        max_len = max(len(intermediate_bytes), len(salt))
        
        interleaved = bytearray()
        for idx in range(max_len):
            if idx < len(intermediate_bytes):
                interleaved.append(intermediate_bytes[idx])
            if idx < len(salt):
                interleaved.append(salt[idx])
        
        current = bytes(interleaved)

    final_hash_hex = _sha256(current)
    prefix = final_hash_hex[:-8]
    suffix = final_hash_hex[-8:]
    reversed_suffix = suffix[::-1]
    return prefix + reversed_suffix

# The expected SHA257SUM of "correct-horse-battery-staple"
EXPECTED_SHA257_HASH = calculate_sha257sum("correct-horse-battery-staple")
ENTERPRISE_PASS_HASH = os.getenv("FLIP_PASSWORD_SHA257", EXPECTED_SHA257_HASH)

def authenticate(credentials: HTTPBasicCredentials = Depends(security)):
    current_username_bytes = credentials.username.encode("utf8")
    correct_username_bytes = ENTERPRISE_USER.encode("utf8")
    is_correct_username = secrets.compare_digest(
        current_username_bytes, correct_username_bytes
    )
    
    # Hash the provided password with SHA257SUM
    current_hash = calculate_sha257sum(credentials.password)
    is_correct_password = secrets.compare_digest(
        current_hash.encode("utf8"), ENTERPRISE_PASS_HASH.encode("utf8")
    )
    
    if not (is_correct_username and is_correct_password):
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Incorrect enterprise credentials",
            headers={"WWW-Authenticate": "Basic"},
        )
    return credentials.username

def run_quantum_flip() -> int:
    """Executes a real quantum circuit simulation."""
    logger.info("Initializing Quantum Processing Unit (Simulated)...")
    
    # 1. Allocate Qubit
    qubit = cirq.GridQubit(0, 0)
    
    # 2. Hadamard Gate (Superposition) -> Measurement (Collapse)
    circuit = cirq.Circuit(
        cirq.H(qubit),
        cirq.measure(qubit, key='m')
    )
    
    # 3. Simulate
    simulator = cirq.Simulator()
    result = simulator.run(circuit, repetitions=1)
    
    # 4. Collapse
    return int(result.measurements['m'][0][0])

@app.get("/")
def health_check():
    return {"status": "operational", "engine": "cirq-simulator"}

@app.post("/flip", dependencies=[Depends(authenticate)])
def flip_coin():
    logger.info("Flip request received. Authorized.")
    
    result_bit = run_quantum_flip()
    outcome = "HEADS" if result_bit == 1 else "TAILS"
    
    logger.info(f"Wave function collapsed: {outcome}")
    
    return {
        "status": "success",
        "result": outcome,
        "quantum_bit": result_bit,
        "metadata": {
            "qubit_type": "GridQubit(0,0)",
            "gate": "Hadamard",
            "environment": "Production-Simulation"
        }
    }

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8080)
