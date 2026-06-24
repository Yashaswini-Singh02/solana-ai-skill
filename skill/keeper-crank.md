# Keeper / Crank (the thin off-chain trigger)

An automated vault still needs something off-chain to *decide when* to rebalance
and to *send* the transaction. This is NOT the analytics "brain" — it is a thin,
permissioned, stateless crank. The program enforces all safety; the keeper only
chooses timing and lands the transaction reliably.

Working TypeScript template: `../templates/keeper/`.

## Responsibilities (keep it minimal)

1. Read pool + position state (via Helius RPC).
2. Evaluate the rebalance trigger policy (threshold-based, see below).
3. Build the instruction, fetch a Jupiter quote if a swap is needed.
4. **Simulate** the transaction and assert success.
5. Set priority fees (Helius), send, and confirm with retries (Helius).
6. Log/emit for the audit trail.

What the keeper must NOT do: hold withdrawal authority over user funds, bypass
guards, or be the source of truth for prices. If the keeper is compromised, the
worst it can do is fail to rebalance or submit txs that the program rejects.

## Trigger policy (cheap, not high-frequency)

```ts
function shouldRebalance(state: PoolState, pos: Position, cfg: Cfg): Action {
  if (priceOutOfRange(pos, state)) return "rebalance";
  if (estimatedILbps(pos, state) > cfg.maxILbps) return "rebalance";
  if (unclaimedFees(pos) > cfg.compoundThreshold) return "compound";
  return "hold";
}
```

## Reliable landing with Helius (the part teams get wrong)

```ts
import { Connection, ComputeBudgetProgram, Transaction } from "@solana/web3.js";

const conn = new Connection(process.env.HELIUS_RPC_URL!, "confirmed");

// 1) Estimate priority fee from Helius
const { priorityFeeEstimate } = await (await fetch(process.env.HELIUS_RPC_URL!, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    jsonrpc: "2.0", id: "1", method: "getPriorityFeeEstimate",
    params: [{ transaction: serializedTx, options: { recommended: true } }],
  }),
})).json().then((r) => r.result);

// 2) Set compute budget + price
tx.add(ComputeBudgetProgram.setComputeUnitLimit({ units: computeUnits }));
tx.add(ComputeBudgetProgram.setComputeUnitPrice({ microLamports: priorityFeeEstimate }));

// 3) SIMULATE before signing — mandatory (golden rule #3)
const sim = await conn.simulateTransaction(tx, { sigVerify: false });
if (sim.value.err) throw new Error(`Sim failed: ${JSON.stringify(sim.value.err)}`);

// 4) Send with retry + fresh blockhash; confirm
async function sendWithRetry(tx: Transaction, signers, tries = 5) {
  for (let i = 0; i < tries; i++) {
    const { blockhash, lastValidBlockHeight } = await conn.getLatestBlockhash("confirmed");
    tx.recentBlockhash = blockhash;
    tx.sign(...signers);
    const sig = await conn.sendRawTransaction(tx.serialize(), { skipPreflight: false, maxRetries: 0 });
    const ok = await conn.confirmTransaction({ signature: sig, blockhash, lastValidBlockHeight }, "confirmed");
    if (!ok.value.err) return sig;
  }
  throw new Error("tx failed to land after retries");
}
```

## Optional: Jito bundle for MEV protection

For rebalance swaps that are sandwich-prone, submit the swap + the LP move as a
Jito bundle so they execute atomically and are harder to sandwich. The program's
oracle-derived `min_out` is still the real protection; the bundle reduces the
attack surface.

## Event-driven trigger (better than polling)

Use Helius webhooks or LaserStream (gRPC) to react to pool/position events
instead of polling on a timer. Subscribe to the DLMM/Whirlpool program and the
vault program; run `shouldRebalance` on relevant events.

## Deployment

- A small cron/worker (Cloudflare Workers Cron, a Vercel Cron function, or a
  plain container). The crank is stateless, so any of these works.
- Store only the keeper keypair (scoped to crank-only authority) as a secret.
- Alert on repeated simulation failures (could indicate a tripped guard / paused
  vault).
