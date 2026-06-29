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
- `tests/` — LiteSVM attack-scenario test stubs (A1–A9) plus runnable invariant
  property tests (`invariants.rs` + `Cargo.toml`).
- `keeper/` — minimal authenticated TypeScript crank using Helius for landing.

These are MIT-licensed like the rest of the kit.

## Building (verified recipe)

The `vault-allocator` program builds to deployable SBF bytecode with Anchor
`0.31.0` + Solana CLI `2.x`/`4.x`.

```bash
cd vault-allocator
cargo check                       # fast host type-check
anchor keys sync                  # replace the placeholder program id with yours
anchor build                      # SBF bytecode -> target/deploy/*.so
```

### Known build snag: `edition2024` / toolchain too old

The Solana SBF platform-tools bundle an older Rust (1.79 in `v1.43`, 1.84 in
`v1.50`) than several current transitive crates, which now require Rust 1.85
(`edition2024`). A clean build today fails with
`feature 'edition2024' is required`. Two fixes:

1. Use newer platform-tools (Rust 1.84) and pin the offending crates:

```bash
cargo update -p proc-macro-crate@3.5.0 --precise 3.2.0
cargo update -p zeroize              --precise 1.8.1
cargo update -p zeroize_derive       --precise 1.4.2
cargo update -p blake3               --precise 1.5.5
cargo update -p indexmap             --precise 2.7.1
cargo update -p unicode-segmentation --precise 1.12.0
anchor build --no-idl -- --tools-version v1.50 -- --locked
```

2. Or commit a `Cargo.lock` with the pins above so downstream builds are
   reproducible.

### Run the invariant tests

```bash
cd tests
cargo test            # property tests I1/I4 over the share-math model
```
