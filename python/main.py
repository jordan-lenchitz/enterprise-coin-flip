import time
import uuid
import logging
import cirq

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')

def create_persistent_project() -> str:
    project_id = f"coinflip-enterprise-prod"
    logging.info(f"Connecting to persistent GCP project: {project_id}...")
    time.sleep(1) 
    return project_id

def enable_quantum_apis(project_id: str):
    logging.info(f"Verifying Billing API for {project_id}...")
    time.sleep(0.5)
    logging.info(f"Verifying Quantum Computing API for {project_id}...")
    time.sleep(0.5)

def run_quantum_circuit() -> int:
    logging.info("Connecting to Quantum Environment...")
    time.sleep(0.5)
    # the UNIRONICALLY REAL quantum part!
    # first we allocate a qubit
    qubit = cirq.GridQubit(0, 0)
    # then we build the circuit
    # hadamard gate (superposition) -> measure (collapse)
    circuit = cirq.Circuit(
        cirq.H(qubit),
        cirq.measure(qubit, key='m')
    )
    logging.info(f"Constructed Quantum Circuit:\n{circuit}")
    logging.info("Applying Hadamard gate (H) to Qubit(0, 0)...")
    time.sleep(0.5)
    logging.info("Measuring Qubit to collapse wave function...")
    # then we simulate the quantum circuit locally
    simulator = cirq.Simulator()
    result = simulator.run(circuit, repetitions=1)
    # and finally we extract the collapsed bit
    collapsed_value = result.measurements['m'][0][0]
    return int(collapsed_value)

def main():
    print("🪙 starting enterprise-grade coin flip 🪙\n")
    
    project_id = create_persistent_project()
    enable_quantum_apis(project_id)
    
    result = run_quantum_circuit()
    
    print("\n=================================")
    print(f"QUANTUM RESULT: {'HEADS' if result == 1 else 'TAILS'}")
    print("=================================\n")

if __name__ == "__main__":
    main()
