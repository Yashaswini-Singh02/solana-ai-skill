# Solana Vault Standard Skill — Agent Configuration

This repository is an AI skill for building secure, automated, SVS-compliant
DeFi vaults on Solana. It is designed for Claude Code, Cursor, Codex, and other
coding agents, and plugs into the Solana AI Kit.

## When to load this skill

Load `skill/SKILL.md` when the user mentions any of:

> vault, SVS, tokenized vault, allocator, ERC-4626 on Solana, yield strategy,
> auto-compound, rebalance, deposit into Meteora / Orca / Raydium, oracle guard,
> keeper / crank, audit my Solana program, migrate a CPI / IDL change.

`skill/SKILL.md` is a **router** — it loads only the focused sub-file(s) needed
for the request. Do not read the whole `skill/` directory at once.

## How this skill is organized

- `skill/` — the progressively-loaded knowledge base (router + focused files).
- `agents/` — `vault-architect` (builds), `vault-auditor` (reviews).
- `commands/` — `/new-vault`, `/audit-vault`, `/migrate-cpi`.
- `rules/` — `vault-safety.md` behavioral lint, applied to all vault code.
- `templates/` — copy-ready Anchor program, guard crate, keeper, tests.

## Golden rules (always enforce)

1. SVS compliance (sRFC 40): build on `@stbr/solana-vault`; don't fork the standard.
2. No swap/LP move without an oracle-deviation guard + oracle-derived `min_out`.
3. Simulate before signing in the keeper.
4. Deposit/per-tx caps + pause circuit-breaker on every vault.
5. Attack matrix (A1-A9) + invariants (I1-I7) must pass before "production-ready".

See `rules/vault-safety.md` for the full enforced list.
