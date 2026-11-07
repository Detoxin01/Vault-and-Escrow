/**
 * LiteSVM + Anchor TypeScript test harness.
 *
 * Instructions are built using the Anchor TS client (`@anchor-lang/core`) and
 * `@solana/spl-token`, then executed directly in-process via `litesvm` (no
 * external validator process required).
 */
import * as anchor from "@anchor-lang/core";
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  AccountLayout,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountIdempotentInstruction,
  createInitializeMint2Instruction,
  createMintToInstruction,
  getAssociatedTokenAddressSync,
  MINT_SIZE,
  MintLayout,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { address, getTransactionDecoder, lamports } from "@solana/kit";
import {
  Clock,
  FailedTransactionMetadata,
  LiteSVM,
  TransactionMetadata,
} from "litesvm";
import { expect } from "chai";

export { TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, LAMPORTS_PER_SOL };
export const BN = anchor.BN;
export type TxResult = TransactionMetadata | FailedTransactionMetadata;

export const TX_FEE = 5_000n;

const kit = (pk: PublicKey) => address(pk.toBase58());

export function loadProgram<T extends anchor.Idl>(idl: T): anchor.Program<T> {
  const dummyWallet = {
    publicKey: Keypair.generate().publicKey,
    signTransaction: async <Tx>(tx: Tx) => tx,
    signAllTransactions: async <Tx>(txs: Tx[]) => txs,
  };
  const provider = new anchor.AnchorProvider(
    new Connection("http://127.0.0.1:8899", "confirmed"),
    dummyWallet as any,
    { commitment: "confirmed" }
  );
  return new anchor.Program<T>(idl, provider);
}

export class Env {
  readonly svm: LiteSVM;
  private readonly decoder = getTransactionDecoder();

  constructor() {
    this.svm = new LiteSVM();
    this.setUnixTimestamp(1_800_000_000n);
  }

  addProgram(programId: PublicKey, soPath: string) {
    this.svm.addProgramFromFile(kit(programId), soPath);
  }

  wallet(sol = 10): Keypair {
    const kp = Keypair.generate();
    this.airdrop(kp.publicKey, sol);
    return kp;
  }

  airdrop(to: PublicKey, sol: number) {
    this.svm.airdrop(kit(to), lamports(BigInt(sol) * BigInt(LAMPORTS_PER_SOL)));
  }

  send(ixs: TransactionInstruction[], signers: Keypair[]): TxResult {
    const seen = new Set<string>();
    const uniqueSigners = signers.filter((s) => {
      const k = s.publicKey.toBase58();
      if (seen.has(k)) return false;
      seen.add(k);
      return true;
    });

    const tx = new Transaction();
    tx.recentBlockhash = this.svm.latestBlockhash();
    tx.feePayer = uniqueSigners[0].publicKey;
    tx.add(...ixs);
    tx.sign(...uniqueSigners);

    const res = this.svm.sendTransaction(this.decoder.decode(tx.serialize()));
    this.svm.expireBlockhash();
    return res;
  }

  now(): bigint {
    return this.svm.getClock().unixTimestamp;
  }

  setUnixTimestamp(ts: bigint) {
    const c: Clock = this.svm.getClock();
    c.unixTimestamp = ts;
    this.svm.setClock(c);
  }

  warp(seconds: number) {
    this.setUnixTimestamp(this.now() + BigInt(seconds));
  }

  balance(pk: PublicKey): bigint {
    return BigInt(this.svm.getBalance(kit(pk)) ?? 0n);
  }

  accountData(pk: PublicKey): Uint8Array | null {
    const acc = this.svm.getAccount(kit(pk));
    if (!acc.exists || acc.lamports === 0n) return null;
    return acc.data;
  }

  owner(pk: PublicKey): PublicKey | null {
    const acc = this.svm.getAccount(kit(pk));
    if (!acc.exists || acc.lamports === 0n) return null;
    return new PublicKey(acc.programAddress);
  }

  exists(pk: PublicKey): boolean {
    return this.accountData(pk) !== null;
  }

  rentExempt(space: number): bigint {
    return this.svm.minimumBalanceForRentExemption(BigInt(space));
  }

  createMint(payer: Keypair, authority: PublicKey, decimals = 6): PublicKey {
    const mint = Keypair.generate();
    expectOk(
      this.send(
        [
          SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: mint.publicKey,
            lamports: Number(this.rentExempt(MINT_SIZE)),
            space: MINT_SIZE,
            programId: TOKEN_PROGRAM_ID,
          }),
          createInitializeMint2Instruction(
            mint.publicKey,
            decimals,
            authority,
            null
          ),
        ],
        [payer, mint]
      )
    );
    return mint.publicKey;
  }

  createAta(payer: Keypair, mint: PublicKey, owner: PublicKey): PublicKey {
    const ata = ataOf(mint, owner);
    expectOk(
      this.send(
        [
          createAssociatedTokenAccountIdempotentInstruction(
            payer.publicKey,
            ata,
            owner,
            mint
          ),
        ],
        [payer]
      )
    );
    return ata;
  }

  mintTo(
    mintAuthority: Keypair,
    mint: PublicKey,
    owner: PublicKey,
    amount: bigint
  ): PublicKey {
    const ata = this.createAta(mintAuthority, mint, owner);
    expectOk(
      this.send(
        [
          createMintToInstruction(
            mint,
            ata,
            mintAuthority.publicKey,
            amount
          ),
        ],
        [mintAuthority]
      )
    );
    return ata;
  }

  tokenBalance(tokenAccount: PublicKey): bigint {
    const data = this.accountData(tokenAccount);
    if (!data) return 0n;
    return AccountLayout.decode(Buffer.from(data)).amount;
  }

  mintSupply(mint: PublicKey): bigint {
    const data = this.accountData(mint);
    if (!data) throw new Error(`mint ${mint.toBase58()} not found`);
    return MintLayout.decode(Buffer.from(data)).supply;
  }
}

export function ataOf(mint: PublicKey, owner: PublicKey): PublicKey {
  return getAssociatedTokenAddressSync(mint, owner, true);
}

export function decodeAccount<T = any>(
  program: anchor.Program<any>,
  name: string,
  data: Uint8Array | null
): T {
  if (!data) throw new Error(`account ${name} does not exist`);
  const buf = Buffer.from(data);
  const candidates = [
    name,
    name[0].toUpperCase() + name.slice(1),
    name[0].toLowerCase() + name.slice(1),
  ];
  let lastErr: unknown;
  for (const candidate of candidates) {
    try {
      return program.coder.accounts.decode(candidate, buf) as T;
    } catch (e) {
      lastErr = e;
    }
  }
  throw lastErr;
}

export function logsOf(res: TxResult): string[] {
  return res instanceof FailedTransactionMetadata
    ? res.meta().logs()
    : res.logs();
}

export function expectOk(res: TxResult): TransactionMetadata {
  if (res instanceof FailedTransactionMetadata) {
    throw new Error(
      `Transaction unexpectedly failed: ${res.toString()}\nLogs:\n${res
        .meta()
        .logs()
        .join("\n")}`
    );
  }
  return res;
}

export function expectErr(res: TxResult, ...codes: string[]) {
  expect(
    res instanceof FailedTransactionMetadata,
    "expected transaction to fail, but it succeeded"
  ).to.eq(true);
  if (codes.length === 0) return;
  const failed = res as FailedTransactionMetadata;
  const combined = `${failed.toString()}\n${failed.meta().logs().join("\n")}`;
  const matched = codes.some((c) => combined.includes(c));
  expect(
    matched,
    `expected one of [${codes.join(", ")}] in failure output:\n${combined}`
  ).to.eq(true);
}
