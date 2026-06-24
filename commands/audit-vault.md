---
name: audit-vault
description: Run the pre-audit checklist on a vault program and emit SECURITY.md.
---

# /audit-vault

Pre-audit gate for an SVS vault. Invoke the `vault-auditor` agent.

## Steps

1. Load `skill/trail-of-bits-checklist.md` and `skill/guards.md`.
2. Inventory instructions, accounts, PDAs, CPIs, and authorities.
3. Walk the checklist (sections 1-9); mark PASS/FAIL/N/A with file:line.
4. Verify invariants I1-I7 exist as tests and attack matrix A1-A9 passes.
5. Emit `SECURITY.md`: invariant statements + guard predicates, the filled
   checklist, and a findings table with severity and remediation.
6. Block mainnet on any open Critical/High finding.
