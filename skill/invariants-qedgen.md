# Formal Invariants (QEDGen / Property-Based)

Turn the security model into machine-checkable properties. Use these as:

1. **QEDGen / formal-verification specs** (where a prover is available), and
2. **Property-based fuzz tests** (always achievable, e.g. with `proptest` or via
   the LiteSVM harness in `attack-tests.md`).

The point: ship invariants alongside the contract so an auditor (Trail of Bits,
QEDGen) verifies properties, not just reads code.

## Core invariants for an SVS allocator vault

### I1. Share-price monotonicity (no free value)

A deposit immediately followed by a withdraw of the same user, in the same
state, never returns more assets than deposited (minus rounding in the vault's
favor).

```text
forall s, a:
  (shares = deposit(a)) ; (out = withdraw(shares))  ==>  out <= a
```

### I2. NAV consistency

`stored_assets` after `sync()` equals idle + sum of venue allocations valued at
the **oracle** price, within a bounded rounding epsilon.

```text
sync(s).stored_assets == idle(s) + Σ_i value_oracle(allocation_i)  ± ε
```

### I3. Conservation (no token creation)

Total asset tokens controlled by the program (idle + deployed) only change by
exactly the net of deposits, withdrawals, swaps, and realized fees. No
instruction mints assets out of thin air.

```text
Δ(controlled_assets) == deposits - withdrawals ± swap_delta + fees_claimed
```

### I4. Share supply ↔ accounting

`share_mint.supply == vault.total_shares` at the end of every instruction.

### I5. Guard dominance (the safety theorem)

No state transition that performs a swap or LP move can succeed if the oracle is
stale, the confidence is too wide, or the pool deviates beyond the band.

```text
forall tx that swaps or moves liquidity:
  success(tx)  ==>  oracle_fresh ∧ oracle_confident ∧ |pool - oracle| <= band
```

This is the formal version of the "anti-thief" guarantee. A manipulated pool can
never produce a successful value-moving transaction.

### I6. Cap & pause respected

```text
forall tx:  success(tx) ==> ¬paused ∧ amount(tx) <= per_tx_cap
forall deposit: success ==> stored_assets_after <= deposit_cap
```

### I7. Authorization

Crank-only instructions succeed only for the configured keeper; admin-only
instructions only for the authority.

## How to express them

### As property tests (always do this)

```rust
proptest! {
    #[test]
    fn deposit_withdraw_never_profits(a in 1u64..1_000_000_000) {
        let mut env = VaultEnv::new();
        let shares = env.deposit(USER, a)?;
        let out = env.withdraw(USER, shares)?;
        prop_assert!(out <= a);                  // I1
        prop_assert_eq!(env.share_supply(), env.total_shares()); // I4
    }
}
```

### As QEDGen-style specs (where supported)

Provide the prover with: the state struct, the transition functions, and the
invariants above as pre/post-conditions. Mark `assert_pool_price_sane`,
`read_oracle`, and the `min_out` floor as the guard predicates that establish I5.

```text
# qedgen.spec (illustrative)
invariant I4: share_mint.supply == vault.total_shares
invariant I5: action.moves_value => guard.passed
preserve I3 across [deposit, withdraw, rebalance_swap, rebalance_meteora, rebalance_orca]
```

## Deliverable

For any vault this skill generates, also generate:

- `tests/invariants.rs` — property tests for I1–I7.
- `SECURITY.md` — prose statement of the invariants + the guard predicates that
  enforce them (hand this to the auditor).

See `trail-of-bits-checklist.md` for the human review gate that complements these
machine checks.
