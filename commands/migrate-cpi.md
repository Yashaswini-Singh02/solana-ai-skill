---
name: migrate-cpi
description: Assisted migration when a venue (Meteora/Orca/Jupiter) ships a breaking IDL change.
---

# /migrate-cpi

Assisted, reviewable migration after a dependency interface change. Never a
silent auto-rewrite of financial code.

## Steps

1. Load `skill/idl-diff-migration.md`.
2. Inputs: the pinned old IDL (`idl/<venue>.<version>.json`) and the new IDL
   (`anchor idl fetch <PROGRAM_ID>`).
3. Diff and classify every change by blast radius (new/required/reordered
   account, renamed ix, struct layout, new arg, removed).
4. Emit `MIGRATION.md` with the classified table + risk tags.
5. Propose minimal diffs to the CPI call sites + account structs.
6. Update `templates/tests/` to target the new IDL and re-run the attack matrix.
7. Stop and request human/auditor approval before any mainnet deploy.

Flag any field whose semantics may have changed even if the name didn't.
