.PHONY: run-go run-python run-rust test-go test-python test-rust db-up db-down db-migrate

db-up:
	docker compose up -d postgres

db-down:
	docker compose down

db-migrate:
	docker compose exec postgres psql -U admin -d coinflip -f /docker-entrypoint-initdb.d/init.sql

run-go:
	docker compose --profile go up --build

run-python:
	docker compose --profile python up --build

run-rust:
	docker compose --profile rust up --build

test-go:
	cd go && go test -v ./...

test-python:
	cd python && pytest -v

test-rust:
	cd rust && cargo test
