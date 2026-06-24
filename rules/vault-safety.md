---
description: Non-negotiable safety rules for any Solana vault code in this repo.
globs:
  - "**/*.rs"
  - "**/programs/**"
  - "**/keeper/**"
alwaysApply: true
---

# Vault Safety Rules (behavioral lint)

Apply these whenever generating or editing vault program or keeper code. If a
change would violate one, stop and fix it before proceeding.

1. SVS compliance: vaults expose `deposit` / `withdraw` / share accounting (and
   `sync` for stored-balance). Do not invent a parallel interface.
2. Guarded value moves: every instruction that swaps, deposits to a venue, or
   moves liquidity MUST call the oracle-deviation guard and (for swaps) enforce
   an oracle-derived `min_out` plus a post-swap delta check. No exceptions.
3. Oracle hygiene: always check staleness and confidence before using a price.
   Value NAV with the oracle, never raw pool spot or raw ATA balances.
4. Caps + pause: every vault has `deposit_cap`, `per_tx_cap`, and an admin
   circuit-breaker checked on every value-moving instruction.
5. Share math: round in the vault's favor; compute against `stored_assets`
   (NAV), and mitigate first-deposit inflation (dead shares).
6. Checked arithmetic only: use `checked_*` / safe wrappers; keep
   `overflow-checks = true` in release.
7. Authorization: crank ops require the keeper signer; admin ops require the
   authority (prefer multisig). Never accept an unsigned arbitrary authority.
8. CPI safety: pin/validate target program ids; scope PDA signer seeds; forward
   Token-2022 transfer-hook remaining accounts.
9. Tests before done: attack matrix A1-A9 and invariants I1-I7 must pass before
   labeling code production-ready.
10. Observability: emit an event on every state change.
