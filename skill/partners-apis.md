# Partner APIs (reference — load on demand)

Quick reference for the ecosystem APIs/SDKs this skill hooks into. Pin versions
and re-verify after upgrades (`idl-diff-migration.md`). All are accessed via
official SDKs or HTTP — zero opaque dependencies.

## Helius — RPC, fees, landing, data

- RPC endpoint: `https://mainnet.helius-rpc.com/?api-key=...` (set `HELIUS_RPC_URL`).
- `getPriorityFeeEstimate` — recommended priority fee for a serialized tx.
- Staked connections / `sendTransaction` — reliable landing.
- Webhooks — push on-chain events (account/program) to your keeper endpoint.
- LaserStream (gRPC) — low-latency streaming for event-driven rebalancing.
- DAS API — asset/token data (useful for share-token metadata, dashboards).

Used by: `keeper-crank.md` (fees, landing, webhooks/LaserStream).

## Jupiter — swap aggregation

- Quote API: `GET https://quote-api.jup.ag/v6/quote?inputMint=&outputMint=&amount=&slippageBps=`
- Swap/instructions API to build the swap; or CPI the aggregator program.
- Use `restrictIntermediateTokens=true`. Treat API slippage as a UX hint only;
  the program enforces an oracle-derived `min_out`.

Used by: `jupiter-rebalance.md`.

## Meteora — DLMM (dynamic liquidity)

- TypeScript SDK + Rust CPI examples (pin versions).
- Pool types: prefer `PermissionlessV2` (Token-2022 + extended config).
- `rebalance_liquidity` (add/remove/shift in one ix), `PositionV2` (~1,400 bins).

Used by: `meteora-dlmm-cpi.md`.

## Orca — Whirlpools (CLMM)

- Whirlpools program + SDK; tick-based concentrated liquidity.
- `open_position`, `increase/decrease_liquidity`, `collect_fees`, `collect_reward`.

Used by: `orca-whirlpool-cpi.md`.

## Raydium — (alternative venue)

- CLMM + CPMM programs; same allocator pattern as Orca/Meteora if you add it as
  a third venue.

## Pyth / Switchboard — oracles

- Pyth pull oracle: `PriceUpdateV2` account; read price, conf, publish_time.
  Verify staleness + confidence (`guards.md`).
- Switchboard On-Demand as an alternative; same guard logic applies.

Used by: `guards.md`, `invariants-qedgen.md`.

## Trail of Bits / QEDGen — security

- Trail of Bits: Solana security guidance + tooling; use the checklist in
  `trail-of-bits-checklist.md` to pre-empt findings.
- QEDGen: formal verification; express invariants from `invariants-qedgen.md` as
  prover specs.

## Solana Foundation / Superteam Brazil — standard & GTM

- SVS / sRFC 40 ownership and review; align to the standard for ecosystem
  adoption and grants.

## Cloudflare / Vercel — keeper hosting

- Cloudflare Workers Cron or Vercel Cron functions to run the stateless crank.
- Edge KV/cache for serving read-only vault stats to a frontend (optional).

## Environment variables (suggested)

```bash
HELIUS_RPC_URL=        # https://mainnet.helius-rpc.com/?api-key=...
JUPITER_BASE_URL=      # https://quote-api.jup.ag/v6
ORACLE_FEED=           # Pyth price update account
KEEPER_KEYPAIR=        # path/secret for crank-only signer
VAULT_ADDRESS=         # target vault
```
