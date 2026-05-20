import cirq
import cirq_ionq
import os
import logging
import hashlib
from fastapi import FastAPI, Depends, HTTPException, status
from fastapi.responses import HTMLResponse
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

EXPECTED_SHA257_HASH = calculate_sha257sum("correct-horse-battery-staple")
ENTERPRISE_PASS_HASH = os.getenv("FLIP_PASSWORD_SHA257", EXPECTED_SHA257_HASH)

def authenticate(credentials: HTTPBasicCredentials = Depends(security)):
    current_username_bytes = credentials.username.encode("utf8")
    correct_username_bytes = ENTERPRISE_USER.encode("utf8")
    is_correct_username = secrets.compare_digest(
        current_username_bytes, correct_username_bytes
    )
    
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

def run_quantum_flip():
    """Executes a real quantum circuit simulation OR hits the physical IonQ hardware."""
    qubit = cirq.GridQubit(0, 0)
    circuit = cirq.Circuit(
        cirq.H(qubit),
        cirq.measure(qubit, key='m')
    )
    
    ionq_key = os.getenv("IONQ_API_KEY")
    
    if ionq_key:
        logger.info("IONQ_API_KEY DETECTED. Connecting to physical IonQ QPU...")
        try:
            service = cirq_ionq.Service(api_key=ionq_key)
            # Send the job to the real quantum processing unit (QPU)
            job = service.create_job(circuit=circuit, repetitions=1, target='qpu')
            logger.info(f"Job created! ID: {job.job_id()}. Waiting in queue (this actually costs money)...")
            
            # This blocks until the physical trapped-ion hardware runs the operation
            result = job.results()
            return int(result.measurements['m'][0][0]), "IonQ QPU (Physical Trapped-Ion Hardware)"
        except Exception as e:
            logger.error(f"Failed to run on physical hardware: {e}")
            raise HTTPException(status_code=500, detail="QPU Hardware Error. The wave function refused to collapse.")
    else:
        logger.info("No IONQ_API_KEY found. Falling back to local simulator.")
        simulator = cirq.Simulator()
        result = simulator.run(circuit, repetitions=1)
        return int(result.measurements['m'][0][0]), "Production-Simulation (Free)"

@app.get("/", response_class=HTMLResponse)
def get_ui():
    ionq_configured = bool(os.getenv("IONQ_API_KEY"))
    status_color = "#00ff00" if ionq_configured else "#ffaa00"
    status_text = "IONQ_API_KEY DETECTED: PHYSICAL HARDWARE ACTIVE" if ionq_configured else "SIMULATOR ACTIVE: MISSING IONQ_API_KEY"
    
    html_content = f"""
    <!DOCTYPE html>
    <html>
    <head>
        <title>Enterprise Quantum Coin Flip</title>
        <style>
            body {{ font-family: monospace; background-color: #0a0a0a; color: #00ff00; text-align: center; padding: 50px; }}
            h1 {{ font-size: 3em; margin-bottom: 0.2em; }}
            .subtitle {{ color: #888; margin-bottom: 40px; }}
            .status-banner {{ background-color: #222; border: 1px solid {status_color}; color: {status_color}; padding: 10px; margin-bottom: 20px; display: inline-block; font-weight: bold; }}
            .cost-box {{ border: 2px dashed #00ff00; display: inline-block; padding: 20px; margin-bottom: 40px; }}
            .cost {{ font-size: 4em; color: #ff0055; margin: 0; }}
            .btn {{ background: #00ff00; color: #000; font-family: monospace; font-size: 2em; padding: 15px 40px; border: none; cursor: pointer; text-transform: uppercase; font-weight: bold; }}
            .btn:hover {{ background: #fff; }}
            #result-area {{ margin-top: 40px; font-size: 2em; min-height: 50px; }}
        </style>
    </head>
    <body>
        <h1>QUANTUM ENTROPY AS A SERVICE</h1>
        <div class="subtitle">B2B Wave Function Collapse</div>
        
        <div class="status-banner">{status_text}</div><br>
        
        <div class="cost-box">
            <p style="margin-top: 0; color: #fff;">Unironic Physical Hardware Execution Cost:</p>
            <p class="cost">$0.33</p>
            <p style="margin-bottom: 0; color: #888;">(IonQ QPU: $0.30 Base Task Fee + $0.03/shot)</p>
        </div>
        <br>
        
        <button class="btn" onclick="flipCoin()">Initiate $0.33 Flip</button>
        
        <div id="result-area"></div>

        <script>
            async function flipCoin() {{
                const resultArea = document.getElementById('result-area');
                resultArea.innerHTML = "Authenticating (SHA257SUM) & Accessing QPU...<br><span style='font-size:0.4em;'>(This may take a few minutes if queueing on physical hardware)</span>";
                
                const pwd = prompt("Enter Enterprise Secret (Warning: You will be charged if physical hardware is active):");
                if (!pwd) {{
                    resultArea.innerHTML = "Aborted.";
                    return;
                }}

                const auth = btoa('ceo:' + pwd);
                
                try {{
                    const response = await fetch('/flip', {{
                        method: 'POST',
                        headers: {{
                            'Authorization': 'Basic ' + auth
                        }}
                    }});
                    
                    if (response.status === 401) {{
                        resultArea.innerHTML = "<span style='color: red;'>Access Denied. SHA257SUM mismatch.</span>";
                        return;
                    }}
                    
                    if (!response.ok) {{
                        resultArea.innerHTML = "<span style='color: red;'>QPU Execution Failed.</span>";
                        return;
                    }}
                    
                    const data = await response.json();
                    resultArea.innerHTML = `RESULT: <strong>${{data.result}}</strong><br><span style='font-size: 0.5em; color: #888;'>Executed on: ${{data.metadata.environment}}</span>`;
                }} catch (e) {{
                    resultArea.innerHTML = "<span style='color: red;'>Error collapsing wave function.</span>";
                }}
            }}
        </script>
    </body>
    </html>
    """
    return HTMLResponse(content=html_content)

@app.post("/flip", dependencies=[Depends(authenticate)])
def flip_coin():
    logger.info("Flip request received. Authorized.")
    
    result_bit, environment = run_quantum_flip()
    outcome = "HEADS" if result_bit == 1 else "TAILS"
    
    logger.info(f"Wave function collapsed: {outcome} on {environment}")
    
    return {
        "status": "success",
        "result": outcome,
        "quantum_bit": result_bit,
        "metadata": {
            "qubit_type": "Trapped-Ion / Simulated GridQubit",
            "gate": "Hadamard",
            "environment": environment
        }
    }

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8080)
