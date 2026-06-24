/**
 * Minimal authenticated crank for an SVS allocator vault.
 *
 * Responsibilities ONLY: decide when to rebalance (threshold policy), build the
 * instruction, fetch a Jupiter quote, SIMULATE, set Helius priority fees, and
 * land the tx with retries. The program enforces every safety check; if this
 * keeper is compromised the worst case is failed/rejected transactions.
 *
 * See skill/keeper-crank.md and skill/jupiter-rebalance.md.
 */
import {
  Connection,
  Keypair,
  Transaction,
  ComputeBudgetProgram,
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";

const HELIUS_RPC_URL = required("HELIUS_RPC_URL");
const JUPITER_BASE_URL = process.env.JUPITER_BASE_URL ?? "https://quote-api.jup.ag/v6";
const POLL_MS = Number(process.env.POLL_MS ?? 30_000);

const conn = new Connection(HELIUS_RPC_URL, "confirmed");
const keeper = loadKeeper();

type Action = "hold" | "rebalance" | "compound";

async function tick() {
  const state = await readVaultAndPoolState();
  const action = decide(state);
  if (action === "hold") return;

  const ixs = await buildInstructions(action, state);
  await sendGuarded(ixs);
}

/** Threshold-based policy. Keep it cheap; do not high-frequency rebalance. */
function decide(s: VaultState): Action {
  if (s.priceOutOfRange) return "rebalance";
  if (s.estimatedILbps > s.cfg.maxILbps) return "rebalance";
  if (s.unclaimedFees > s.cfg.compoundThreshold) return "compound";
  return "hold";
}

/** Build the program instruction(s); for swaps, derive min_out from a Jupiter quote. */
async function buildInstructions(action: Action, s: VaultState): Promise<TransactionInstruction[]> {
  if (action !== "rebalance") {
    // TODO: build compound instruction
    return [];
  }
  const quote = await jupiterQuote(s.inMint, s.outMint, s.amountIn, s.cfg.slippageBps);
  const minOut = BigInt(quote.outAmount); // program re-checks this against the oracle
  // TODO: assemble your program's rebalance_swap ix with (amountIn, minOut) and
  // the Jupiter route accounts as remaining accounts.
  void minOut;
  return [];
}

/** Simulate -> priority fee -> send with retry. */
async function sendGuarded(ixs: TransactionInstruction[]) {
  if (ixs.length === 0) return;
  const tx = new Transaction().add(...ixs);

  // Priority fee from Helius.
  const fee = await heliusPriorityFee(tx);
  tx.add(ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 }));
  tx.add(ComputeBudgetProgram.setComputeUnitPrice({ microLamports: fee }));

  // MANDATORY: simulate before signing.
  const { blockhash } = await conn.getLatestBlockhash("confirmed");
  tx.recentBlockhash = blockhash;
  tx.feePayer = keeper.publicKey;
  const sim = await conn.simulateTransaction(tx);
  if (sim.value.err) throw new Error(`sim failed: ${JSON.stringify(sim.value.err)}`);

  await sendWithRetry(tx);
}

async function sendWithRetry(tx: Transaction, tries = 5): Promise<string> {
  for (let i = 0; i < tries; i++) {
    const { blockhash, lastValidBlockHeight } = await conn.getLatestBlockhash("confirmed");
    tx.recentBlockhash = blockhash;
    tx.sign(keeper);
    const sig = await conn.sendRawTransaction(tx.serialize(), { skipPreflight: false, maxRetries: 0 });
    const res = await conn.confirmTransaction({ signature: sig, blockhash, lastValidBlockHeight }, "confirmed");
    if (!res.value.err) return sig;
  }
  throw new Error("transaction failed to land after retries");
}

async function heliusPriorityFee(tx: Transaction): Promise<number> {
  const serialized = tx.serialize({ requireAllSignatures: false, verifySignatures: false }).toString("base64");
  const r = await fetch(HELIUS_RPC_URL, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: "1",
      method: "getPriorityFeeEstimate",
      params: [{ transaction: serialized, options: { recommended: true } }],
    }),
  });
  const j = await r.json();
  return Math.max(1, Math.floor(j?.result?.priorityFeeEstimate ?? 1));
}

async function jupiterQuote(inMint: string, outMint: string, amount: bigint, slippageBps: number) {
  const url = `${JUPITER_BASE_URL}/quote?inputMint=${inMint}&outputMint=${outMint}` +
    `&amount=${amount}&slippageBps=${slippageBps}&restrictIntermediateTokens=true`;
  const r = await fetch(url);
  if (!r.ok) throw new Error(`jupiter quote failed: ${r.status}`);
  return r.json();
}

// ---- stubs you implement against your deployed program ----
type Cfg = { maxILbps: number; compoundThreshold: number; slippageBps: number };
type VaultState = {
  cfg: Cfg;
  priceOutOfRange: boolean;
  estimatedILbps: number;
  unclaimedFees: number;
  inMint: string;
  outMint: string;
  amountIn: bigint;
};

async function readVaultAndPoolState(): Promise<VaultState> {
  // TODO: fetch vault account + Meteora/Orca position + oracle via Helius RPC.
  return {
    cfg: { maxILbps: 100, compoundThreshold: 0, slippageBps: 50 },
    priceOutOfRange: false,
    estimatedILbps: 0,
    unclaimedFees: 0,
    inMint: "",
    outMint: "",
    amountIn: 0n,
  };
}

function required(name: string): string {
  const v = process.env[name];
  if (!v) throw new Error(`missing env ${name}`);
  return v;
}

function loadKeeper(): Keypair {
  const secret = process.env.KEEPER_SECRET_KEY;
  if (!secret) throw new Error("missing env KEEPER_SECRET_KEY (JSON array of bytes)");
  return Keypair.fromSecretKey(Uint8Array.from(JSON.parse(secret)));
}

async function main() {
  // eslint-disable-next-line no-console
  console.log("keeper started; vault:", process.env.VAULT_ADDRESS);
  void PublicKey; // referenced when you build real instructions
  for (;;) {
    try {
      await tick();
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("tick error:", e);
    }
    await new Promise((r) => setTimeout(r, POLL_MS));
  }
}

main();
