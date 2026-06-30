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
  property tests for **I1–I7** (`invariants.rs` + `Cargo.toml`).
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

The Solana SBF platform-tools bundle an older Rust (Cargo **1.79**, even in the
2.x platform-tools the Anza `stable` installer ships) than several current
transitive crates, which now require Rust 1.85 (`edition2024`). A *fresh*
resolve fails to even parse their manifests with
`feature 'edition2024' is required`.

**This is solved by the committed `Cargo.lock`** in `vault-allocator/`. It pins
every edition2024 crate back to its last pre-2024 release, so `anchor build`
must be run with `--locked` (CI does this) and the lock must **not** be
regenerated:

```bash
anchor build --no-idl -- -- --locked   # uses the committed Cargo.lock
```

> Don't pass `--tools-version v1.50` to a 2.x toolchain — that bundle id no
> longer resolves and `cargo-build-sbf` panics with `NotFound`. Use the tools
> the installer ships; pin crates via the lock instead of downgrading the
> toolchain.

#### Refreshing the lock

If you bump a dependency and need to regenerate, the pins below reproduce a
1.79-parseable graph (verify with `cargo +1.79.0 metadata --locked`). The set
drifts as upstream publishes new edition2024 releases; the trickiest are pulled
in transitively by the **optional** `pyth` SDK via a second `anchor-lang`:

```bash
cargo generate-lockfile
cargo update -p blake3                --precise 1.5.5   # digest 0.11 -> 0.10 (block-buffer 0.12 -> 0.10.4)
cargo update -p zeroize               --precise 1.8.1
cargo update -p zeroize_derive        --precise 1.4.2
cargo update -p anchor-lang@1.1.2     --precise 0.31.1  # drop the pyth-pulled anchor 1.x
cargo update -p proc-macro-crate@3.5.0 --precise 3.2.0  # toml_edit 0.23 -> 0.22 (drops toml_parser)
cargo update -p indexmap@2.14.0       --precise 2.7.1   # hashbrown 0.17 -> 0.15
```

The `@version` specs target whatever a fresh resolve picks today — adjust the
left-hand version if cargo reports a different one. When a *new*
`feature 'edition2024' is required` appears for some `<crate>`, find its puller
with `cargo tree -i <crate>` and pin the closest parent back, then re-validate
with `cargo +1.79.0 metadata --locked` and commit the updated `Cargo.lock`.

### Run the tests

```bash
cd guards && cargo test   # attack matrix A1–A9 over the guard cores
cd ../tests && cargo test # invariant property tests I1–I7
```

Both run in CI on every push/PR (`.github/workflows/ci.yml`); they are
dependency-light and finish in seconds, so the "tested" claim is reproducible.
