/**
 * Attack-scenario tests (A1-A9) for the SVS allocator vault, using LiteSVM.
 * See skill/attack-tests.md. These are stubs: implement `setupVault` and the
 * env helpers against your built program, then assert each exploit reverts.
 *
 * Run with: `pnpm add -D litesvm vitest && pnpm vitest`
 */
import { describe, test, expect, beforeEach } from "vitest";
// import { LiteSVM } from "litesvm";

type Env = {
  setOraclePrice: (p: number) => void;
  setPoolPrice: (p: number) => void;
  warpClock: (secs: number) => void;
  setOracleConfBps: (bps: number) => void;
  oracleFloor: () => bigint;
  buildRebalanceIx: (o: { minOut: bigint }) => unknown;
  sendWithKeeper: (ix: unknown) => { err?: unknown; logs: string };
  sendWithSigner: (signer: string, ix: unknown) => { err?: unknown; logs: string };
  deposit: (user: string, amount: bigint) => bigint; // returns shares
  donateToVaultAta: (amount: bigint) => void;
};

// TODO: implement using LiteSVM + your program bytes.
function setupVault(): Env {
  throw new Error("implement setupVault() against your built program");
}

describe("SVS allocator vault - attack matrix", () => {
  let env: Env;
  beforeEach(() => {
    env = setupVault();
  });

  test("control: in-band rebalance succeeds", () => {
    env.setOraclePrice(1.0);
    env.setPoolPrice(1.005); // within 1% band
    const res = env.sendWithKeeper(env.buildRebalanceIx({ minOut: env.oracleFloor() }));
    expect(res.err).toBeUndefined();
  });

  test("A1: manipulated pool price reverts", () => {
    env.setOraclePrice(1.0);
    env.setPoolPrice(1.2); // 20% off, band is 1%
    const res = env.sendWithKeeper(env.buildRebalanceIx({ minOut: env.oracleFloor() }));
    expect(res.logs).toContain("PriceManipulated");
  });

  test("A2: loose min_out reverts (sandwich protection)", () => {
    env.setOraclePrice(1.0);
    env.setPoolPrice(1.0);
    const tooLow = env.oracleFloor() - 1n;
    const res = env.sendWithKeeper(env.buildRebalanceIx({ minOut: tooLow }));
    expect(res.logs).toContain("SlippageTooLoose");
  });

  test("A3: stale oracle reverts", () => {
    env.setOraclePrice(1.0);
    env.warpClock(10_000); // exceed max_staleness_secs
    const res = env.sendWithKeeper(env.buildRebalanceIx({ minOut: env.oracleFloor() }));
    expect(res.logs).toContain("StaleOracle");
  });

  test("A4: wide oracle confidence reverts", () => {
    env.setOraclePrice(1.0);
    env.setOracleConfBps(500); // wider than max_conf_bps
    const res = env.sendWithKeeper(env.buildRebalanceIx({ minOut: env.oracleFloor() }));
    expect(res.logs).toContain("OracleUncertain");
  });

  test("A5: first-deposit inflation does not steal from victim", () => {
    const attackerShares = env.deposit("attacker", 1n);
    env.donateToVaultAta(1_000_000n); // inflate raw ATA balance
    const victimShares = env.deposit("victim", 1_000_000n);
    // Victim must receive fair shares; attacker cannot redeem most of victim's deposit.
    expect(victimShares).toBeGreaterThan(0n);
    expect(attackerShares).toBe(1n);
  });

  test("A6: unauthorized crank reverts", () => {
    const res = env.sendWithSigner("not_keeper", env.buildRebalanceIx({ minOut: env.oracleFloor() }));
    expect(res.logs).toContain("Unauthorized");
  });
});
