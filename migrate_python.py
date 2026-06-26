import os
import re

with open("python/app.py", "r") as f:
    content = f.read()

# Add imports
imports_to_add = """from fastapi import FastAPI, Depends, HTTPException, status, BackgroundTasks
import httpx
from database import SessionLocal, Account, Ledger"""

content = content.replace("from fastapi import FastAPI, Depends, HTTPException, status", imports_to_add)

# Replace authenticate function
old_auth = """def authenticate(credentials: HTTPBasicCredentials = Depends(security)):
    current_username_bytes = credentials.username.encode("utf8")
    correct_username_bytes = ENTERPRISE_USER.encode("utf8")
    is_correct_username = secrets.compare_digest(current_username_bytes, correct_username_bytes)
    current_hash = calculate_sha257sum(credentials.password)
    is_correct_password = secrets.compare_digest(current_hash.encode("utf8"), ENTERPRISE_PASS_HASH.encode("utf8"))
    if not (is_correct_username and is_correct_password):
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Incorrect enterprise credentials", headers={"WWW-Authenticate": "Basic"})
    return credentials.username"""

new_auth = """def authenticate(credentials: HTTPBasicCredentials = Depends(security)):
    db = SessionLocal()
    try:
        account = db.query(Account).filter(Account.username == credentials.username).first()
        if not account:
            raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Incorrect enterprise credentials", headers={"WWW-Authenticate": "Basic"})
        current_hash = calculate_sha257sum(credentials.password)
        if not secrets.compare_digest(current_hash.encode("utf8"), account.password_hash.encode("utf8")):
            raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Incorrect enterprise credentials", headers={"WWW-Authenticate": "Basic"})
        return account
    finally:
        db.close()"""

content = content.replace(old_auth, new_auth)

# Add batch_quantum_flip_worker and /flip/batch route
new_code = """
async def batch_quantum_flip_worker(count: int, account_id: int, webhook_url: str):
    logger.info(f"Starting batch quantum flip worker for account_id={account_id}, count={count}")
    results = []
    
    ionq_key = os.getenv("IONQ_API_KEY")
    environment = "Production-Simulation (Free)"
    
    qubit = cirq.GridQubit(0, 0)
    circuit = cirq.Circuit(cirq.H(qubit), cirq.measure(qubit, key='m'))
    
    if ionq_key:
        environment = "IonQ Aria Physical QPU ($12.42 Flat Fee)"
        try:
            service = cirq_ionq.Service(api_key=ionq_key)
            job = service.create_job(circuit=circuit, repetitions=count, target='qpu.aria')
            logger.info(f"Batch Job created! ID: {job.job_id()}")
            result = job.results()
            measurements = result.measurements['m']
            results = [int(m[0]) for m in measurements]
        except Exception as e:
            logger.error(f"Failed to run batch on physical hardware: {e}")
            return
    else:
        simulator = cirq.Simulator()
        result = simulator.run(circuit, repetitions=count)
        measurements = result.measurements['m']
        results = [int(m[0]) for m in measurements]

    # Save to ledger
    db = SessionLocal()
    try:
        total_cost = 12.42 if ionq_key else 0.0
        for r in results:
            outcome = "HEADS" if r == 1 else "TAILS"
            entry = Ledger(account_id=account_id, environment=environment, cost=total_cost/count, result=outcome)
            db.add(entry)
        db.commit()
    except Exception as e:
        logger.error(f"DB Error: {e}")
    finally:
        db.close()
        
    # Send webhook
    if webhook_url:
        logger.info(f"Firing webhook to {webhook_url}")
        async with httpx.AsyncClient() as client:
            try:
                payload = {
                    "status": "success",
                    "batch_size": count,
                    "results": ["HEADS" if r == 1 else "TAILS" for r in results],
                    "raw_bits": results,
                    "environment": environment
                }
                await client.post(webhook_url, json=payload, timeout=10.0)
                logger.info("Webhook fired successfully")
            except Exception as e:
                logger.error(f"Failed to fire webhook: {e}")

from pydantic import BaseModel
class BatchRequest(BaseModel):
    count: int

@app.post("/flip/batch", status_code=202)
def flip_coin_batch(req: BatchRequest, background_tasks: BackgroundTasks, account: Account = Depends(authenticate)):
    logger.info(f"Batch flip request received. Count: {req.count}. Authorized user: {account.username}")
    
    if req.count <= 0 or req.count > 1000:
        raise HTTPException(status_code=400, detail="Count must be between 1 and 1000")
        
    background_tasks.add_task(batch_quantum_flip_worker, req.count, account.id, account.webhook_url)
    
    return {
        "status": "accepted",
        "message": f"Batch quantum flip for {req.count} wave functions queued.",
        "webhook_target": account.webhook_url
    }
"""

content = content.replace("def flip_coin():", new_code + "\ndef flip_coin(account: Account = Depends(authenticate)):")

# Replace run_quantum_flip call in flip_coin to use account
content = content.replace("def flip_coin(account: Account = Depends(authenticate)):\n    logger.info(\"Flip request received. Authorized.\")", 
                          "def flip_coin(account: Account = Depends(authenticate)):\n    logger.info(f\"Flip request received. Authorized user: {account.username}\")")


# also need to write to file
with open("python/app.py", "w") as f:
    f.write(content)
print("Migration completed successfully.")
