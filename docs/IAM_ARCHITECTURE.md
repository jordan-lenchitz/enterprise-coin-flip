# iam architecture

notes on how identity, provisioning, and access reviews work for the coin flip service.

because each physical wave-function collapse on the IonQ Aria QPU costs $12.42, unauthenticated coin flipping is a financial liability. the service sits behind Okta Workforce (internal employees synced from Workday) and Auth0 (external B2B actuarial/reinsurance partners), with a local 35-round `sha257sum` basic auth fallback in case both identity providers are down and an executive still needs a 50/50 decision made.

```
Workday HRIS                  External B2B Partners
     |                                  |
     v                                  v
Okta Workforce (infra/okta.tf)   Auth0 CIAM (infra/auth0.tf)
     |              |                   |
  SCIM 2.0     OIDC Bearer         OIDC Bearer
     |              |                   |
     +--------------+-------------------+
                    |
                    v
         enterprise-coin-flip
       (src/iam.rs, src/scim.rs)
```

## authentication (`src/iam.rs`)

requests to `/flip` and `/flip/batch` accept either:

- `Authorization: Bearer <jwt>` signed with `HS256` (`OIDC_JWT_SECRET`) from a trusted issuer (`https://jordanlenchitz.okta.com/oauth2/default`, `https://identity.jordanlenchitz.com/`, or local `/oauth2/token`) with audience `api://quantum-coin-flip`.
- `Authorization: Basic <base64>` verified against the `accounts` table in postgres using 35 rounds of `sha257sum`.

in both paths, if the principal matches a row in `accounts` where `active = false`, the request is rejected with `403 Forbidden` regardless of whether the JWT has expired yet.

for local testing without an Okta tenant, `POST /oauth2/token` mints a token for a preset persona (`executive`, `analyst`, `b2b_partner`, `scim_admin`) or custom claims.

## authorization & qpu gating

`evaluate_flip_policy()` checks role membership first, then attributes:

1. **role check**: caller must hold `Quantum-Executives`, `Quantum-Analysts`, or `CIAM-B2B-Partner`.
2. **batch quota**: `POST /flip/batch` checks `requested_shots <= max_batch_shots` (default 50 for analysts, 250 for b2b partners, 1000 for executives).
3. **hardware qpu gate ($12.42)**: physical IonQ Aria execution requires:
   - `qpu_entitled = true`
   - a non-empty `cost_center` (for chargeback to the appropriate department)
   - MFA in `amr` (`mfa`, `otp`, `hwk`, `webauthn`, or `push`) when `requested_shots > 1`
   - an `org_id` claim if the caller is a `CIAM-B2B-Partner`

if a caller is allowed to flip a coin but fails the hardware QPU gate, the request quietly downgrades to the $0.00 local pure-rust state vector simulator unless the caller explicitly passed `X-Require-Hardware-QPU: true`, in which case it returns `403 Forbidden`.

## scim 2.0 provisioning (`src/scim.rs`)

okta lifecycle management pushes workday changes to `/scim/v2/Users` using `SCIM_BEARER_TOKEN` (or an OIDC token with `SCIM-Provisioner`):

- `POST /scim/v2/Users` (joiner): upserts the account in postgres with `external_id`, `department`, `cost_center`, `roles`, `qpu_entitled`, and `max_batch_shots`. logs `SCIM_JOINER_PROVISIONED`.
- `PATCH /scim/v2/Users/{id}` (mover / leaver): updates roles, cost center, or batch limits in place on department transfers (`SCIM_MOVER_UPDATED`), or sets `active = false` on termination (`SCIM_LEAVER_DEPROVISIONED`).
- `DELETE /scim/v2/Users/{id}` (leaver): soft-deletes the user (`active = false`, `qpu_entitled = false`, sets `deprovisioned_at`). rows in `ledger` are kept intact so finance can still reconcile prior $12.42 coin flips during SOC 2 audits.

## access reviews (`/governance/*`)

- `GET /governance/access-review`: returns all accounts, their current entitlements, total flips and cumulative USD spend from `ledger`, separation-of-duties findings (e.g. an inactive user still marked `qpu_entitled`, or hardware QPU entitlement with no `cost_center` to bill), and the last 25 rows of `iam_audit_log`.
- `POST /governance/certify`: records an `APPROVE` or `REVOKE` decision for an `account_id`. `REVOKE` sets `qpu_entitled = false` and clamps `max_batch_shots` to `10`.

## metrics (`/metrics`)

- `iam_authn_requests_total{method, issuer, outcome}`
- `iam_authz_decisions_total{policy_layer, decision, reason}`
- `scim_provisioning_events_total{operation, lifecycle_stage}`
- `governance_certifications_total{decision}`
