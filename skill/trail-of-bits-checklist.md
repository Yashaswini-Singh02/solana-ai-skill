# Pre-Audit Checklist (Trail of Bits style)

Run this before paying for a full audit and before mainnet. It mirrors the
issues that Solana auditors (Trail of Bits, QEDGen, etc.) find most often. The
goal: turn a multi-week audit cycle into a fast self-serve gate, then hand a
clean report to the auditor.

Output a filled `SECURITY.md` with each item marked PASS / FAIL / N/A and a note.

## 1. Account validation (the #1 source of Solana exploits)

- [ ] Every account has explicit ownership constraints (`#[account(owner = ...)]`
      or program checks). No `UncheckedAccount` without a documented reason.
- [ ] All PDAs are derived with `seeds = [...]` + `bump` and validated, not
      passed in unchecked.
- [ ] `has_one` / key equality checks on every relationship
      (vault ↔ authority, vault ↔ share_mint, position ↔ vault_authority).
- [ ] Token accounts: mint and owner constrained (no arbitrary ATA substitution).
- [ ] No "account confusion": discriminators checked (Anchor does this; verify
      for any manual deserialization).

## 2. Signer & authority

- [ ] Admin instructions require the authority signer (multisig recommended).
- [ ] Crank instructions require the keeper signer (`guards.md` Guard 5).
- [ ] Upgrade authority is a multisig or burned; documented in `SECURITY.md`.
- [ ] No instruction lets a caller pass an arbitrary `authority` they don't sign.

## 3. Arithmetic

- [ ] All math is checked (`checked_add/sub/mul/div`) or uses safe wrappers.
- [ ] Share math rounds in the vault's favor (down on deposit shares, down on
      withdraw assets).
- [ ] No precision loss that lets `shares == 0` for a non-trivial deposit.
- [ ] First-deposit inflation mitigated (dead shares / NAV from `stored_assets`).

## 4. Oracle & pricing (the "anti-thief" core)

- [ ] Oracle staleness checked (`max_staleness_secs`).
- [ ] Oracle confidence checked (`max_conf_bps`).
- [ ] Pool spot validated against oracle band before any swap/LP move.
- [ ] Swap `min_out` is oracle-derived, not route-derived; post-swap delta verified.
- [ ] NAV uses oracle prices, never raw pool spot or raw ATA balances.

## 5. CPI safety

- [ ] CPI target program IDs are pinned/validated (no arbitrary program CPI).
- [ ] PDA signer seeds are scoped to exactly the needed authority.
- [ ] Remaining accounts for Token-2022 transfer hooks are forwarded (not stripped).
- [ ] Reentrancy considered: state updated before/after CPI consistently; no
      double-spend via callback.

## 6. Funds safety & limits

- [ ] `deposit_cap` and `per_tx_cap` enforced.
- [ ] Circuit-breaker `pause` exists and is tested.
- [ ] Withdrawals cannot be blocked by a single failing venue (graceful path).
- [ ] No path leaves dust/value stranded in a closed position (fees claimed first).

## 7. Token-2022 specifics

- [ ] Transfer-fee extension accounted for in amounts (received != sent).
- [ ] Transfer hooks supported or explicitly rejected at deposit.
- [ ] Confidential-transfer mints handled or rejected (SVS-3/4 path).

## 8. Testing & verification

- [ ] Invariants I1–I7 implemented as property tests (`invariants-qedgen.md`).
- [ ] Attack-scenario tests pass (`attack-tests.md`): manipulation, sandwich,
      stale oracle, first-deposit, unauthorized crank.
- [ ] Tests run on a forked/realistic state (LiteSVM with real pool accounts).

## 9. Operational

- [ ] Events emitted for deposit/withdraw/rebalance/pause (audit trail).
- [ ] Keeper failure does not risk funds (only delays rebalancing).
- [ ] Runbook for pausing and for an oracle outage.
- [ ] Dependencies pinned; `cargo audit` clean; no unmaintained crates.

## Severity triage

When something FAILs, classify:

- **Critical**: direct fund loss/theft (missing oracle guard, missing owner check).
- **High**: fund lockup, share dilution, unauthorized control.
- **Medium**: incorrect accounting within bounds, griefing.
- **Low/Informational**: events, docs, gas.

Block mainnet on any open Critical/High.
