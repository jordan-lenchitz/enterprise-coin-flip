CREATE TABLE IF NOT EXISTS accounts (
    id SERIAL PRIMARY KEY,
    username VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    webhook_url VARCHAR(255)
);

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

INSERT INTO accounts (username, password_hash, webhook_url)
VALUES ('ceo', '18bb824a4ad1f39be49cc91af302dad50e27f9af7ff17b5dade977dc3beb0a58', 'http://localhost:8080/webhook-test')
ON CONFLICT (username) DO NOTHING;
