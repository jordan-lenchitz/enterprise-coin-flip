CREATE TABLE IF NOT EXISTS accounts (
    id SERIAL PRIMARY KEY,
    username VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    webhook_url VARCHAR(255),
    external_id VARCHAR(255),
    email VARCHAR(255),
    active BOOLEAN NOT NULL DEFAULT TRUE,
    department VARCHAR(100) NOT NULL DEFAULT 'Executive',
    cost_center VARCHAR(100) NOT NULL DEFAULT 'CC-1001-EXEC',
    roles TEXT NOT NULL DEFAULT 'Quantum-Executives',
    qpu_entitled BOOLEAN NOT NULL DEFAULT TRUE,
    max_batch_shots INTEGER NOT NULL DEFAULT 1000,
    idp_issuer VARCHAR(255) NOT NULL DEFAULT 'local-breakglass',
    deprovisioned_at TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE accounts ADD COLUMN IF NOT EXISTS external_id VARCHAR(255);
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS email VARCHAR(255);
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS active BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS department VARCHAR(100) NOT NULL DEFAULT 'Executive';
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS cost_center VARCHAR(100) NOT NULL DEFAULT 'CC-1001-EXEC';
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS roles TEXT NOT NULL DEFAULT 'Quantum-Executives';
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS qpu_entitled BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS max_batch_shots INTEGER NOT NULL DEFAULT 1000;
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS idp_issuer VARCHAR(255) NOT NULL DEFAULT 'local-breakglass';
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS deprovisioned_at TIMESTAMP;
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP;

CREATE TABLE IF NOT EXISTS api_keys (
    id SERIAL PRIMARY KEY,
    account_id INTEGER REFERENCES accounts(id) ON DELETE CASCADE,
    api_key VARCHAR(255) UNIQUE NOT NULL,
    service VARCHAR(255) NOT NULL, -- e.g., 'ionq'
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS ledger (
    id SERIAL PRIMARY KEY,
    account_id INTEGER REFERENCES accounts(id) ON DELETE SET NULL,
    environment VARCHAR(50) NOT NULL, -- 'Simulator' or 'IonQ QPU'
    cost DECIMAL(10, 4) NOT NULL,
    result VARCHAR(10) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS iam_audit_log (
    id SERIAL PRIMARY KEY,
    account_id INTEGER REFERENCES accounts(id) ON DELETE SET NULL,
    actor VARCHAR(255) NOT NULL,
    event_type VARCHAR(100) NOT NULL, -- e.g., 'SCIM_JOINER', 'SCIM_MOVER', 'SCIM_LEAVER', 'OIG_CERTIFICATION', 'ABAC_DENY'
    lifecycle_stage VARCHAR(50) NOT NULL, -- 'JOINER', 'MOVER', 'LEAVER', 'GOVERNANCE', 'AUTHZ'
    details TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO accounts (
    username,
    password_hash,
    webhook_url,
    external_id,
    email,
    active,
    department,
    cost_center,
    roles,
    qpu_entitled,
    max_batch_shots,
    idp_issuer
)
VALUES (
    'ceo',
    '18bb824a4ad1f39be49cc91af302dad50e27f9af7ff17b5dade977dc3beb0a58',
    'http://localhost:8080/webhook-test',
    'wd-emp-00001',
    'ceo@jordanlenchitz.com',
    TRUE,
    'Executive Leadership',
    'CC-1001-EXEC',
    'Quantum-Executives,IAM-Governance-Admins',
    TRUE,
    1000,
    'https://jordanlenchitz.okta.com/oauth2/default'
)
ON CONFLICT (username) DO NOTHING;
