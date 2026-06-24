# Templates

Copy-ready skeletons referenced by the skill. They are intentionally minimal and
annotated; replace the pseudo-CPI calls with the pinned venue SDK/CPI crates and
fill in account contexts before deploying. Nothing here is audited — run the
attack tests and the checklist in `skill/trail-of-bits-checklist.md` first.

- `vault-allocator/` — SVS-9 allocator Anchor program (idle reserve + Meteora +
  Orca child allocations, Jupiter ratio rebalancing), with guards wired into
  every value-moving instruction.
- `guards/` — the reusable security middleware as a standalone crate (the same
  logic is vendored inside `vault-allocator/.../guards.rs`).
- `tests/` — LiteSVM attack-scenario test stubs (A1–A9).
- `keeper/` — minimal authenticated TypeScript crank using Helius for landing.

These are MIT-licensed like the rest of the kit.
