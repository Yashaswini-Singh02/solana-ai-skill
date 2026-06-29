# Solana Vault Standard Skill

[![CI](https://github.com/Yashaswini-Singh02/solana-ai-skill/actions/workflows/ci.yml/badge.svg)](https://github.com/Yashaswini-Singh02/solana-ai-skill/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A production-grade, token-efficient AI skill that turns a coding agent (Claude
Code, Cursor, Codex) into an expert at building **secure, automated,
SVS-compliant DeFi vaults** on Solana.

Most Solana AI tooling builds "the brains" (off-chain analytics, dashboards,
alerts). This skill builds **"the muscle"**: the on-chain smart contract that
custodies funds, deploys them into liquidity venues (Meteora, Orca), and
rebalances safely without getting drained or front-run.

It is built directly on the **Solana Vault Standard (SVS, sRFC 40)** — the
ERC-4626 equivalent for Solana, maintained by Superteam Brazil and under Solana
Foundation review: <https://github.com/solanabr/solana-vault-standard>.

## What it does

1. **Blueprint generator** — scaffolds SVS-compliant vaults (focus: SVS-9
   allocator and SVS-8 multi-asset basket) with correct PDAs, share accounting,
   and venue CPIs.
2. **Anti-exploit guards** — a reusable security middleware (oracle deviation,
   TWAP-vs-spot, oracle-derived `min_out`, deposit/per-tx caps, circuit-breaker)
   wired into every value-moving instruction.
3. **Pre-audit + formal invariants** — a Trail-of-Bits-style checklist plus
   machine-checkable invariants (QEDGen / property tests) and an attack matrix.
4. **Keeper/crank lifecycle** — a thin, permissioned off-chain trigger with
   reliable transaction landing via Helius.
5. **Assisted IDL-diff migration** — reviewable upgrades when a venue ships a
   breaking interface change.

## Architecture (progressive loading)

The entry point `skill/SKILL.md` is a **router**. It reads only the focused
sub-file(s) that match the request, so context loads on demand.

```text
.
├── CLAUDE.md                 # when/how to load this skill
├── install.sh                # standard installer (defaults)
├── install-custom.sh         # custom installer (full options)
├── install-common.sh         # shared installer helpers
├── skill/
│   ├── SKILL.md              # router (entry point)
│   ├── svs-variant-picker.md
│   ├── meteora-dlmm-cpi.md
│   ├── orca-whirlpool-cpi.md
│   ├── jupiter-rebalance.md
│   ├── guards.md
│   ├── invariants-qedgen.md
│   ├── trail-of-bits-checklist.md
│   ├── keeper-crank.md
│   ├── attack-tests.md
│   ├── idl-diff-migration.md
│   ├── svs-interface.md      # on-demand reference
│   └── partners-apis.md      # on-demand reference
├── agents/                   # vault-architect, vault-auditor
├── commands/                 # /new-vault, /audit-vault, /migrate-cpi
├── rules/                    # vault-safety.md behavioral lint
└── templates/                # Anchor program, guard crate, keeper, tests
```

## Install

The installer builds a self-contained, **auto-discoverable** skill at
`<config>/skills/solana-vault-standard/SKILL.md`, so agentic IDEs register it
natively — Claude Code (`.claude/skills/`) and Cursor (`.cursor/skills/`). A
`CLAUDE.md` router is also dropped at the project root as a universal fallback.

```bash
# Standard (registers the skill for Claude Code + Cursor)
./install.sh /path/to/your/project

# Custom (pick IDEs and what to bundle)
./install-custom.sh --target /path/to/your/project --ide cursor --no-templates
```

Then in your agent: `/new-vault` to scaffold, `/audit-vault` before mainnet.

## Ecosystem partners

| Layer | Partner | Where |
| ----- | ------- | ----- |
| Standard / GTM | Solana Foundation, Superteam Brazil (SVS, sRFC 40) | `skill/svs-interface.md` |
| Liquidity venues | Meteora, Orca, Raydium | `skill/meteora-dlmm-cpi.md`, `skill/orca-whirlpool-cpi.md` |
| Swap routing | Jupiter | `skill/jupiter-rebalance.md` |
| RPC / fees / landing / data | Helius | `skill/keeper-crank.md`, `skill/partners-apis.md` |
| Security | Trail of Bits, QEDGen | `skill/trail-of-bits-checklist.md`, `skill/invariants-qedgen.md` |
| Oracles | Pyth, Switchboard | `skill/guards.md` |
| Keeper hosting | Cloudflare, Vercel | `skill/keeper-crank.md` |

## Golden rules

1. SVS compliance — build on `@stbr/solana-vault`, don't fork the standard.
2. No swap/LP move without an oracle-deviation guard + oracle-derived `min_out`.
3. Simulate before signing (keeper).
4. Caps + pause on every vault.
5. Attack matrix (A1-A9) + invariants (I1-I7) pass before "production-ready".

## Testing

The security logic is runnable and tested in CI (`.github/workflows/ci.yml`):

- `templates/guards/` — attack matrix **A1–A9** over the guard cores (`cargo test`).
- `templates/tests/` — property tests for invariants **I1–I7** (`cargo test`).
- `templates/keeper/` — type-checked (`tsc --noEmit`).

By default the program reads a self-contained oracle feed so it builds with zero
external dependencies; build `--features pyth` for a real Pyth pull-oracle
(`PriceUpdateV2`) read (see `skill/guards.md`).

## Safety & status

The `templates/` are unaudited skeletons with annotated pseudo-CPIs. Pin the
venue SDK/CPI versions you test against, fill in account contexts, and run the
attack tests + checklist before any deployment.

## License

MIT. See [LICENSE](LICENSE). The Solana Vault Standard is a separate
open-source project; verify and honor its upstream license when vendoring its
code.
