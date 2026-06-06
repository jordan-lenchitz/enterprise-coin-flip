import cirq
import cirq_ionq
import os
import logging
import hashlib
import time
from fastapi import FastAPI, Depends, HTTPException, status
from fastapi.responses import HTMLResponse
from fastapi.security import HTTPBasic, HTTPBasicCredentials
import secrets

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger("enterprise-coin-flip")

app = FastAPI(
    title="Enterprise Quantum Coin Flip Service",
    description="Secure, authenticated B2B solution for true quantum bit collapse.",
    version="1.1.0"
)

security = HTTPBasic()

ENTERPRISE_USER = os.getenv("FLIP_USER", "ceo")

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

EXPECTED_SHA257_HASH = calculate_sha257sum("111111111111111111111")
ENTERPRISE_PASS_HASH = os.getenv("FLIP_PASSWORD_SHA257", EXPECTED_SHA257_HASH)

def authenticate(credentials: HTTPBasicCredentials = Depends(security)):
    current_username_bytes = credentials.username.encode("utf8")
    correct_username_bytes = ENTERPRISE_USER.encode("utf8")
    is_correct_username = secrets.compare_digest(current_username_bytes, correct_username_bytes)
    current_hash = calculate_sha257sum(credentials.password)
    is_correct_password = secrets.compare_digest(current_hash.encode("utf8"), ENTERPRISE_PASS_HASH.encode("utf8"))
    if not (is_correct_username and is_correct_password):
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Incorrect enterprise credentials", headers={"WWW-Authenticate": "Basic"})
    return credentials.username

def run_quantum_flip():
    """executes a real quantum circuit simulation OR hits the physical IonQ Aria hardware"""
    qubit = cirq.GridQubit(0, 0)
    circuit = cirq.Circuit(cirq.H(qubit), cirq.measure(qubit, key='m'))
    ionq_key = os.getenv("IONQ_API_KEY")
    
    if ionq_key:
        logger.info("IONQ_API_KEY DETECTED. Connecting to physical IonQ ARIA (Capped $12.42)...")
        try:
            service = cirq_ionq.Service(api_key=ionq_key)
            # explicitly target 'qpu.aria' to lock in the $12.42 price point
            job = service.create_job(circuit=circuit, repetitions=1, target='qpu.aria')
            logger.info(f"Job created! ID: {job.job_id()}. Physical atoms are now being manipulated...")
            result = job.results()
            return int(result.measurements['m'][0][0]), "IonQ Aria Physical QPU ($12.42 Flat Fee)"
        except Exception as e:
            logger.error(f"Failed to run on physical hardware: {e}")
            raise HTTPException(status_code=500, detail="QPU Hardware Error. Wave function refused to collapse.")
    else:
        logger.info("No IONQ_API_KEY found. Falling back to local simulator.")
        simulator = cirq.Simulator()
        result = simulator.run(circuit, repetitions=1)
        return int(result.measurements['m'][0][0]), "Production-Simulation (Free)"

@app.get("/", response_class=HTMLResponse)
def get_ui():
    ionq_configured = bool(os.getenv("IONQ_API_KEY"))
    status_color = "#00ff00" if ionq_configured else "#ffaa00"
    status_text = "IONQ_API_KEY ACTIVE: PHYSICAL ARIA QPU TARGETED" if ionq_configured else "SIMULATOR ACTIVE: NO API KEY DETECTED"
    
    html_content = f"""
    <!DOCTYPE html>
    <html>
    <head>
        <title>Enterprise Quantum Coin Flip</title>
        <style>
            body {{ font-family: 'Courier New', monospace; background-color: #050505; color: #00ff00; text-align: center; padding: 50px; overflow-x: hidden; }}
            h1 {{ font-size: 3.5em; margin-bottom: 0.1em; letter-spacing: -2px; }}
            .subtitle {{ color: #555; margin-bottom: 40px; text-transform: uppercase; }}
            .status-banner {{ background-color: #111; border: 1px solid {status_color}; color: {status_color}; padding: 10px 20px; margin-bottom: 20px; display: inline-block; font-size: 0.8em; }}
            .cost-box {{ border: 2px solid #00ff00; background: #001100; display: inline-block; padding: 30px; margin-bottom: 40px; box-shadow: 0 0 20px #00ff0033; }}
            .cost {{ font-size: 5em; color: #ff0055; margin: 10px 0; font-weight: bold; }}
            .btn {{ background: #00ff00; color: #000; font-family: monospace; font-size: 1.8em; padding: 20px 60px; border: none; cursor: pointer; text-transform: uppercase; font-weight: bold; transition: all 0.2s; }}
            .btn:hover {{ background: #fff; transform: scale(1.05); }}
            .btn:disabled {{ background: #333; color: #666; cursor: not-allowed; }}
            #ledger {{ text-align: left; max-width: 500px; margin: 40px auto; border-left: 2px solid #333; padding-left: 20px; min-height: 200px; color: #888; font-size: 0.9em; }}
            .ledger-entry {{ margin-bottom: 5px; animation: fadeIn 0.5s; }}
            .ledger-cost {{ float: right; color: #ff0055; }}
            @keyframes fadeIn {{ from {{ opacity: 0; transform: translateX(-10px); }} to {{ opacity: 1; transform: translateX(0); }} }}
            #final-result {{ font-size: 4em; font-weight: bold; margin-top: 20px; color: #fff; text-shadow: 0 0 20px #00ff00; }}
        </style>
    </head>
    <body>
        <div class="status-banner">{status_text}</div><br>
        <h1>QUANTUM ENTROPY</h1>
        <div class="subtitle">B2B Hardware-Level Logic Termination</div>
        
        <div class="cost-box">
            <div style="color: #fff; font-size: 1.2em;">GUARANTEED ARIA QPU FLAT FEE:</div>
            <div class="cost">$12.42</div>
            <div style="color: #888;">(Per successful wave function collapse)</div>
        </div>
        <br>
        
        <button id="flip-btn" class="btn" onclick="flipCoin()">Initiate Physical Flip</button>
        
        <div id="ledger"></div>
        <div id="final-result"></div>

        <script>
            let ledgerEntries = [
                {{ t: 0, txt: "Authenticating SHA257SUM protocol...", c: "$0.00" }},
                {{ t: 1, txt: "Establishing Vertex AI Quantum Tunnel...", c: "$0.00" }},
                {{ t: 3, txt: "SUBMITTED: Task creation fee (GCP)", c: "$0.30" }},
                {{ t: 5, txt: "HARDWARE LOCK: IonQ Aria Reserved", c: "$12.12" }},
                {{ t: 8, txt: "Cryogenic stabilization initiated...", c: "---" }},
                {{ t: 12, txt: "Pumping vacuum to 10^-10 Torr...", c: "---" }},
                {{ t: 16, txt: "Laser lattice alignment (355nm)...", c: "---" }},
                {{ t: 22, txt: "Ion trapping: Ytterbium-171 isolated", c: "---" }},
                {{ t: 28, txt: "Applying Hadamard microwave pulse...", c: "---" }},
                {{ t: 35, txt: "SUPERPOSITION ACHIEVED (0 & 1)", c: "---" }},
                {{ t: 40, txt: "Awaiting physical photon emission...", c: "---" }},
                {{ t: 50, txt: "Measuring state (Collapsing world-line)", c: "---" }},
                {{ t: 60, txt: "Processing 1-bit entropy results...", c: "---" }}
            ];

            async function flipCoin() {{
                const pwd = prompt("ENTER ENTERPRISE SECRET (111111111111111111111):");
                if (!pwd) return;

                const btn = document.getElementById('flip-btn');
                const ledger = document.getElementById('ledger');
                const resultDiv = document.getElementById('final-result');
                
                btn.disabled = true;
                ledger.innerHTML = "";
                resultDiv.innerHTML = "";

                let startTime = Date.now();
                let entryIdx = 0;

                const ticker = setInterval(() => {{
                    let elapsed = (Date.now() - startTime) / 1000;
                    if (entryIdx < ledgerEntries.length && elapsed >= ledgerEntries[entryIdx].t) {{
                        const entry = ledgerEntries[entryIdx];
                        ledger.innerHTML += `<div class='ledger-entry'>> ${{entry.txt}} <span class='ledger-cost'>${{entry.c}}</span></div>`;
                        entryIdx++;
                    }}
                }}, 500);

                const auth = btoa('ceo:' + pwd);
                try {{
                    const response = await fetch('/flip', {{
                        method: 'POST',
                        headers: {{ 'Authorization': 'Basic ' + auth }}
                    }});
                    
                    clearInterval(ticker);
                    
                    if (response.status === 401) {{
                        ledger.innerHTML += "<div class='ledger-entry' style='color:red;'>> AUTH_FAILURE: SHA257 mismatch.</div>";
                        btn.disabled = false;
                        return;
                    }}
                    
                    const data = await response.json();
                    
                    // Fill remaining ledger
                    for(; entryIdx < ledgerEntries.length; entryIdx++) {{
                         const entry = ledgerEntries[entryIdx];
                         ledger.innerHTML += `<div class='ledger-entry'>> ${{entry.txt}} <span class='ledger-cost'>${{entry.c}}</span></div>`;
                    }}

                    ledger.innerHTML += `<div class='ledger-entry' style='color:#fff;'>> SUCCESS: Entropy verified via ${{data.metadata.environment}}</div>`;
                    resultDiv.innerHTML = data.result;
                    
                }} catch (e) {{
                    clearInterval(ticker);
                    ledger.innerHTML += "<div class='ledger-entry' style='color:red;'>> QPU_FATAL: Connection severed.</div>";
                }}
                btn.disabled = false;
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
    return {{
        "status": "success",
        "result": outcome,
        "quantum_bit": result_bit,
        "metadata": {{
            "qubit_type": "IonQ Aria physical trapped-ion",
            "gate": "Hadamard (H)",
            "environment": environment
        }}
    }}

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8080)
