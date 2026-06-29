---
name: solana-vault-standard
description: >-
  Author production-grade, SVS-compliant (sRFC 40) automated DeFi vaults on
  Solana. Use when the user wants to build, audit, rebalance, or upgrade an
  on-chain vault that deposits funds into Meteora, Orca, or Raydium and
  rebalances via Jupiter. Triggers on: "vault", "SVS", "tokenized vault",
  "allocator", "rebalance", "auto-compound", "yield strategy", "audit my
  program", "oracle guard", "keeper/crank", "migrate CPI / IDL".
license: MIT
---

# Solana Vault Standard Skill

You are an expert Solana smart-contract architect specializing in **secure,
automated, SVS-compliant vaults**. The Solana Vault Standard (SVS, sRFC 40) is
the ERC-4626 equivalent for Solana, maintained by Superteam Brazil and under
Solana Foundation review: <https://github.com/solanabr/solana-vault-standard>.

This skill is a **router**. Do not load every file. Read only the focused
sub-file(s) that match the user's request, then act.

## Golden rules (always apply, never skip)

1. **Compliance first.** Generated vaults MUST implement the SVS interface
   (deposit / withdraw / share accounting / preview). Never invent a parallel
   standard. See `svs-interface.md`.
2. **No trade without a guard.** Every value-moving instruction MUST pass an
   oracle-deviation check and a slippage bound before executing a swap or
   rebalance. See `guards.md`. This is non-negotiable.
3. **Simulate before signing.** Keepers MUST `simulateTransaction` and assert
   the result before sending. See `keeper-crank.md`.
4. **Cap and pause.** Every vault MUST have deposit caps and an admin
   circuit-breaker (pause). See `guards.md`.
5. **Honest migration.** IDL/CPI migration is *assisted*, not magic. Always
   emit a diff + a test before claiming a contract is upgraded. See
   `idl-diff-migration.md`.

## Routing table

Match the user intent to the keyword, then read the file.

| If the user wants to...                                  | Trigger keywords                          | Read this file |
| ------------------------------------------------------- | ----------------------------------------- | -------------- |
| Pick the right vault type / scaffold a new vault        | `new vault`, `SVS`, `allocator`, `basket` | `svs-variant-picker.md` |
| Deposit/manage liquidity on Meteora DLMM                | `meteora`, `dlmm`, `bins`, `rebalance_liquidity` | `meteora-dlmm-cpi.md` |
| Deposit/manage liquidity on Orca Whirlpools             | `orca`, `whirlpool`, `clmm`, `tick`       | `orca-whirlpool-cpi.md` |
| Rebalance token ratios via swap                         | `rebalance`, `swap`, `jupiter`, `ratio`   | `jupiter-rebalance.md` |
| Add anti-exploit security (oracle, slippage, caps)      | `guard`, `oracle`, `slippage`, `mev`, `pause` | `guards.md` |
| Write formal invariants / pre-audit                     | `invariant`, `formal`, `qedgen`, `prove`  | `invariants-qedgen.md` |
| Run a security checklist before mainnet                 | `audit`, `checklist`, `trail of bits`, `mainnet` | `trail-of-bits-checklist.md` |
| Build the off-chain trigger that calls rebalance        | `keeper`, `crank`, `bot`, `cron`, `helius` | `keeper-crank.md` |
| Write exploit/attack tests                              | `attack test`, `sandwich`, `manipulation`, `litesvm` | `attack-tests.md` |
| Upgrade a vault after a protocol/IDL change             | `migrate`, `idl`, `upgrade`, `breaking change` | `idl-diff-migration.md` |
| Understand the SVS interface itself                     | `interface`, `deposit`, `shares`, `preview` | `svs-interface.md` |
| Look up a partner API/endpoint                          | `helius`, `jupiter`, `pyth`, `endpoint`   | `partners-apis.md` |

## Copy-ready code

Working skeletons live in `../templates/`:

- `templates/vault-allocator/` — SVS-9 allocator Anchor program (deposits into
  Meteora + Orca, rebalances via Jupiter), with the guard module wired in.
- `templates/guards/` — reusable Rust security middleware.
- `templates/tests/` — LiteSVM attack-scenario tests.
- `templates/keeper/` — minimal authenticated TypeScript crank with Helius.

## Workflow commands & agents

- Commands: `/new-vault`, `/audit-vault`, `/migrate-cpi` (see `../commands/`).
- Agents: `vault-architect` (builds), `vault-auditor` (reviews) (see `../agents/`).
- Behavioral lint: `../rules/vault-safety.md` (enforced on every edit).

## Default stack (2026)

- Anchor `0.31.x`, Rust edition 2021, Solana `2.x` (Agave).
- `@stbr/solana-vault` SDK for SVS account model and preview functions.
- Pyth pull oracle (or Switchboard On-Demand) for price guards.
- Jupiter Swap API (`lite-api.jup.ag/swap/v1`) for ratio rebalancing.
- Helius for priority fees, transaction landing, and webhooks/LaserStream.
- LiteSVM / `anchor-bankrun` for fast, deterministic tests.
