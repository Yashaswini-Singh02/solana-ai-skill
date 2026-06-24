---
name: new-vault
description: Scaffold a new SVS-compliant allocator vault.
---

# /new-vault

Scaffold a production-grade SVS vault.

## Steps

1. Load `skill/SKILL.md` and `skill/svs-variant-picker.md`.
2. Ask the user (or infer): deposit asset, target venues (Meteora/Orca/Raydium),
   rebalance policy, deposit/per-tx caps, oracle feed, keeper pubkey.
3. Copy `templates/vault-allocator/` into the user's workspace.
4. Fill `InitializeParams` (caps + `GuardConfig`).
5. Add the requested venue CPIs and the Jupiter rebalance instruction.
6. Wire guards into every value-moving instruction.
7. Generate `tests/` (attack matrix A1-A9) and `invariants.rs` (I1-I7).
8. Print a checklist of what remains before mainnet (run `/audit-vault`).

Never finish without guards on every value-moving instruction.
