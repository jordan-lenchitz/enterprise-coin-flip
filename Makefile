IAAC ?= $(shell command -v tofu 2>/dev/null || echo terraform)

.PHONY: run test db-up db-down db-migrate infra-init infra-fmt infra-validate infra-plan infra-apply infra-destroy

db-up:
	docker compose up -d postgres

db-down:
	docker compose down

db-migrate:
	docker compose exec postgres psql -U admin -d coinflip -f /docker-entrypoint-initdb.d/init.sql

run:
	docker compose up --build

test:
	cargo test

infra-init:
	$(IAAC) -chdir=infra init

infra-fmt:
	$(IAAC) -chdir=infra fmt -recursive

infra-validate:
	$(IAAC) -chdir=infra validate

infra-plan:
	$(IAAC) -chdir=infra plan

infra-apply:
	$(IAAC) -chdir=infra apply

infra-destroy:
	$(IAAC) -chdir=infra destroy
