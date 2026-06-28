.PHONY: run-rust test-rust db-up db-down db-migrate

db-up:
	docker compose up -d postgres

db-down:
	docker compose down

db-migrate:
	docker compose exec postgres psql -U admin -d coinflip -f /docker-entrypoint-initdb.d/init.sql

run-rust:
	docker compose --profile rust up --build

test-rust:
	cargo test
