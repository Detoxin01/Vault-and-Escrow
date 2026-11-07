import fs from "fs";
import path from "path";
import { expect } from "chai";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import type { AnchorEscrow } from "../target/types/anchor_escrow";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  ataOf,
  BN,
  decodeAccount,
  Env,
  expectErr,
  expectOk,
  loadProgram,
  TOKEN_PROGRAM_ID,
} from "./utils";

const idl = JSON.parse(
  fs.readFileSync(path.resolve(__dirname, "../target/idl/anchor_escrow.json"), "utf8")
);
const SO_PATH = path.resolve(__dirname, "../target/deploy/anchor_escrow.so");
const DECIMALS = 6;
const UNIT = 1_000_000n; // 1 token with 6 decimals

describe("anchor-escrow: make / take / refund / update + Timed Conditional Escrow (TypeScript + LiteSVM)", () => {
  const program = loadProgram<AnchorEscrow>(idl as AnchorEscrow);
  const programId = program.programId;

  function findEscrow(maker: PublicKey, seed: bigint) {
    const seedBuf = Buffer.alloc(8);
    seedBuf.writeBigUInt64LE(seed);
    const [escrow, bump] = PublicKey.findProgramAddressSync(
      [Buffer.from("escrow"), maker.toBuffer(), seedBuf],
      programId
    );
    return { escrow, bump };
  }

  function setup() {
    const env = new Env();
    env.addProgram(programId, SO_PATH);

    const mintAuthority = env.wallet(10);
    const maker = env.wallet(10);
    const taker = env.wallet(10);
    const verifier = env.wallet(5);
    const crank = env.wallet(5);

    const mintA = env.createMint(mintAuthority, mintAuthority.publicKey, DECIMALS);
    const mintB = env.createMint(mintAuthority, mintAuthority.publicKey, DECIMALS);

    const makerAtaA = env.mintTo(mintAuthority, mintA, maker.publicKey, 1_000n * UNIT);
    const takerAtaB = env.mintTo(mintAuthority, mintB, taker.publicKey, 1_000n * UNIT);

    return {
      env,
      mintAuthority,
      maker,
      taker,
      verifier,
      crank,
      mintA,
      mintB,
      makerAtaA,
      takerAtaB,
    };
  }

  async function ixMake(params: {
    maker: PublicKey;
    mintA: PublicKey;
    mintB: PublicKey;
    seed: bigint;
    deposit: bigint;
    receive: bigint;
    unlockDelay?: number;
    duration?: number;
    verifier?: PublicKey | null;
  }) {
    const { escrow } = findEscrow(params.maker, params.seed);
    const vault = ataOf(params.mintA, escrow);
    return program.methods
      .make(
        new BN(params.seed.toString()),
        new BN(params.deposit.toString()),
        new BN(params.receive.toString()),
        new BN(params.unlockDelay ?? 0),
        new BN(params.duration ?? 0),
        params.verifier ?? null
      )
      .accountsPartial({
        maker: params.maker,
        mintA: params.mintA,
        mintB: params.mintB,
        makerAtaA: ataOf(params.mintA, params.maker),
        escrow,
        vault,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
  }

  async function ixTake(params: {
    taker: PublicKey;
    maker: PublicKey;
    mintA: PublicKey;
    mintB: PublicKey;
    seed: bigint;
    expectedReceive: bigint;
    verifier?: PublicKey | null;
  }) {
    const { escrow } = findEscrow(params.maker, params.seed);
    return program.methods
      .take(new BN(params.expectedReceive.toString()))
      .accountsPartial({
        taker: params.taker,
        maker: params.maker,
        verifier: params.verifier ?? null,
        mintA: params.mintA,
        mintB: params.mintB,
        takerAtaA: ataOf(params.mintA, params.taker),
        takerAtaB: ataOf(params.mintB, params.taker),
        makerAtaB: ataOf(params.mintB, params.maker),
        escrow,
        vault: ataOf(params.mintA, escrow),
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
  }

  async function ixRefund(params: {
    caller: PublicKey;
    maker: PublicKey;
    mintA: PublicKey;
    seed: bigint;
  }) {
    const { escrow } = findEscrow(params.maker, params.seed);
    return program.methods
      .refund()
      .accountsPartial({
        caller: params.caller,
        maker: params.maker,
        mintA: params.mintA,
        makerAtaA: ataOf(params.mintA, params.maker),
        escrow,
        vault: ataOf(params.mintA, escrow),
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
  }

  async function ixUpdate(params: {
    maker: PublicKey;
    escrow?: PublicKey;
    seed: bigint;
    newReceive: bigint;
    newUnlockDelay?: number;
    newDuration?: number;
    newVerifier?: PublicKey | null;
  }) {
    const escrow = params.escrow ?? findEscrow(params.maker, params.seed).escrow;
    return program.methods
      .update(
        new BN(params.newReceive.toString()),
        new BN(params.newUnlockDelay ?? 0),
        new BN(params.newDuration ?? 0),
        params.newVerifier ?? null
      )
      .accountsPartial({
        maker: params.maker,
        escrow,
      })
      .instruction();
  }

  async function ixConfirmDelivery(verifier: PublicKey, escrow: PublicKey) {
    return program.methods
      .confirmDelivery()
      .accountsPartial({
        verifier,
        escrow,
      })
      .instruction();
  }

  it("make: creates Escrow PDA, locks Mint A in vault, and validates inputs", async () => {
    const { env, maker, verifier, mintA, mintB, makerAtaA } = setup();
    const seed = 1n;
    const deposit = 100n * UNIT;
    const receive = 250n * UNIT;

    // Validation checks: zero deposit/receive, same mint, verifier without expiry, unlock >= expiry
    expectErr(
      env.send(
        [await ixMake({ maker: maker.publicKey, mintA, mintB, seed, deposit: 0n, receive })],
        [maker]
      ),
      "InvalidAmount"
    );
    expectErr(
      env.send(
        [await ixMake({ maker: maker.publicKey, mintA, mintB: mintA, seed, deposit, receive })],
        [maker]
      ),
      "SameMint"
    );
    expectErr(
      env.send(
        [
          await ixMake({
            maker: maker.publicKey,
            mintA,
            mintB,
            seed,
            deposit,
            receive,
            duration: 0,
            verifier: verifier.publicKey,
          }),
        ],
        [maker]
      ),
      "VerifierRequiresExpiry"
    );
    expectErr(
      env.send(
        [
          await ixMake({
            maker: maker.publicKey,
            mintA,
            mintB,
            seed,
            deposit,
            receive,
            unlockDelay: 500,
            duration: 500,
          }),
        ],
        [maker]
      ),
      "InvalidTimeWindow"
    );

    // Valid make
    const now = env.now();
    expectOk(
      env.send(
        [
          await ixMake({
            maker: maker.publicKey,
            mintA,
            mintB,
            seed,
            deposit,
            receive,
            unlockDelay: 60,
            duration: 3600,
          }),
        ],
        [maker]
      )
    );

    const { escrow } = findEscrow(maker.publicKey, seed);
    const vault = ataOf(mintA, escrow);
    expect(env.tokenBalance(vault)).to.eq(deposit);
    expect(env.tokenBalance(makerAtaA)).to.eq(900n * UNIT);

    const state = decodeAccount<any>(program, "escrow", env.accountData(escrow));
    expect(state.maker.toBase58()).to.eq(maker.publicKey.toBase58());
    expect(BigInt(state.receive.toString())).to.eq(receive);
    expect(BigInt(state.createdAt.toString())).to.eq(now);
    expect(BigInt(state.unlockAt.toString())).to.eq(now + 60n);
    expect(BigInt(state.expiresAt.toString())).to.eq(now + 3600n);
    expect(state.verifier).to.eq(null);
    expect(state.deliveryConfirmed).to.eq(true);
  });

  it("take: atomically swaps Mint A & Mint B and closes vault + escrow", async () => {
    const { env, maker, taker, mintA, mintB } = setup();
    const seed = 2n;
    const deposit = 150n * UNIT;
    const receive = 300n * UNIT;

    expectOk(
      env.send(
        [await ixMake({ maker: maker.publicKey, mintA, mintB, seed, deposit, receive })],
        [maker]
      )
    );

    const { escrow } = findEscrow(maker.publicKey, seed);
    const vault = ataOf(mintA, escrow);
    const makerSolBefore = env.balance(maker.publicKey);

    expectOk(
      env.send(
        [
          await ixTake({
            taker: taker.publicKey,
            maker: maker.publicKey,
            mintA,
            mintB,
            seed,
            expectedReceive: receive,
          }),
        ],
        [taker]
      )
    );

    expect(env.tokenBalance(ataOf(mintA, taker.publicKey))).to.eq(deposit);
    expect(env.tokenBalance(ataOf(mintB, maker.publicKey))).to.eq(receive);
    expect(env.tokenBalance(ataOf(mintB, taker.publicKey))).to.eq(700n * UNIT);
    expect(env.exists(escrow)).to.eq(false);
    expect(env.exists(vault)).to.eq(false);
    // Maker received the rent back from both closed accounts
    expect(env.balance(maker.publicKey) > makerSolBefore).to.eq(true);
  });

  it("update: lets maker revise terms and protects taker from front-running", async () => {
    const { env, maker, taker, mintA, mintB } = setup();
    const seed = 3n;
    const deposit = 100n * UNIT;
    const initialReceive = 200n * UNIT;
    const updatedReceive = 350n * UNIT;

    expectOk(
      env.send(
        [
          await ixMake({
            maker: maker.publicKey,
            mintA,
            mintB,
            seed,
            deposit,
            receive: initialReceive,
          }),
        ],
        [maker]
      )
    );

    const { escrow } = findEscrow(maker.publicKey, seed);

    // Non-maker cannot update
    expectErr(
      env.send(
        [
          await ixUpdate({
            maker: taker.publicKey,
            escrow,
            seed,
            newReceive: 1n,
          }),
        ],
        [taker]
      )
    );

    // Maker updates price to 350 and adds a 2-hour expiry
    expectOk(
      env.send(
        [
          await ixUpdate({
            maker: maker.publicKey,
            seed,
            newReceive: updatedReceive,
            newDuration: 7200,
          }),
        ],
        [maker]
      )
    );

    // Taker with stale expectedReceive (200) is protected from overpaying
    expectErr(
      env.send(
        [
          await ixTake({
            taker: taker.publicKey,
            maker: maker.publicKey,
            mintA,
            mintB,
            seed,
            expectedReceive: initialReceive,
          }),
        ],
        [taker]
      ),
      "ReceiveMismatch"
    );

    // Taker agreeing to updated price (350) settles cleanly
    expectOk(
      env.send(
        [
          await ixTake({
            taker: taker.publicKey,
            maker: maker.publicKey,
            mintA,
            mintB,
            seed,
            expectedReceive: updatedReceive,
          }),
        ],
        [taker]
      )
    );
    expect(env.tokenBalance(ataOf(mintB, maker.publicKey))).to.eq(updatedReceive);
  });

  it("refund: allows maker to cancel anytime on unconditional escrow, blocks strangers before expiry", async () => {
    const { env, maker, crank, mintA, mintB, makerAtaA } = setup();
    const seed = 4n;
    const deposit = 250n * UNIT;

    expectOk(
      env.send(
        [
          await ixMake({
            maker: maker.publicKey,
            mintA,
            mintB,
            seed,
            deposit,
            receive: 500n * UNIT,
            duration: 3600,
          }),
        ],
        [maker]
      )
    );

    // Stranger cannot refund before expiry
    expectErr(
      env.send(
        [
          await ixRefund({
            caller: crank.publicKey,
            maker: maker.publicKey,
            mintA,
            seed,
          }),
        ],
        [crank]
      ),
      "NotExpired"
    );

    // Maker refunds and gets full deposit back
    expectOk(
      env.send(
        [
          await ixRefund({
            caller: maker.publicKey,
            maker: maker.publicKey,
            mintA,
            seed,
          }),
        ],
        [maker]
      )
    );

    const { escrow } = findEscrow(maker.publicKey, seed);
    expect(env.tokenBalance(makerAtaA)).to.eq(1_000n * UNIT);
    expect(env.exists(escrow)).to.eq(false);
  });

  it("timed escrow (Clock sysvar): enforces unlock cliff, blocks expired takes, and allows permissionless crank refund", async () => {
    const { env, maker, taker, crank, mintA, mintB, makerAtaA } = setup();
    const seed = 5n;
    const deposit = 400n * UNIT;
    const receive = 400n * UNIT;

    // Lock for 10 minutes (600s), expire after 1 hour (3600s)
    expectOk(
      env.send(
        [
          await ixMake({
            maker: maker.publicKey,
            mintA,
            mintB,
            seed,
            deposit,
            receive,
            unlockDelay: 600,
            duration: 3600,
          }),
        ],
        [maker]
      )
    );

    // At t = +120s -> still locked!
    env.warp(120);
    expectErr(
      env.send(
        [
          await ixTake({
            taker: taker.publicKey,
            maker: maker.publicKey,
            mintA,
            mintB,
            seed,
            expectedReceive: receive,
          }),
        ],
        [taker]
      ),
      "EscrowLocked"
    );

    // Warp past expiry (t = +3601s total) -> take now fails with EscrowExpired!
    env.warp(3500);
    expectErr(
      env.send(
        [
          await ixTake({
            taker: taker.publicKey,
            maker: maker.publicKey,
            mintA,
            mintB,
            seed,
            expectedReceive: receive,
          }),
        ],
        [taker]
      ),
      "EscrowExpired"
    );

    // Permissionless crank refunds expired escrow -> tokens & rent go strictly back to maker!
    expectOk(
      env.send(
        [
          await ixRefund({
            caller: crank.publicKey,
            maker: maker.publicKey,
            mintA,
            seed,
          }),
        ],
        [crank]
      )
    );
    expect(env.tokenBalance(makerAtaA)).to.eq(1_000n * UNIT);
    expect(env.tokenBalance(ataOf(mintA, crank.publicKey))).to.eq(0n);
  });

  it("conditional timed escrow (Mode A — confirm_delivery): requires verifier attestation, blocks maker rugging, and settles in window", async () => {
    const { env, maker, taker, verifier, crank, mintA, mintB } = setup();
    const seed = 6n;
    const deposit = 200n * UNIT;
    const receive = 200n * UNIT;

    expectOk(
      env.send(
        [
          await ixMake({
            maker: maker.publicKey,
            mintA,
            mintB,
            seed,
            deposit,
            receive,
            unlockDelay: 60,
            duration: 3600,
            verifier: verifier.publicKey,
          }),
        ],
        [maker]
      )
    );

    const { escrow } = findEscrow(maker.publicKey, seed);
    env.warp(120); // Past unlock_at

    // Taker cannot take before delivery is confirmed
    expectErr(
      env.send(
        [
          await ixTake({
            taker: taker.publicKey,
            maker: maker.publicKey,
            mintA,
            mintB,
            seed,
            expectedReceive: receive,
          }),
        ],
        [taker]
      ),
      "ConditionNotMet"
    );

    // Impostor cannot confirm delivery
    expectErr(
      env.send([await ixConfirmDelivery(crank.publicKey, escrow)], [crank]),
      "UnauthorizedVerifier"
    );

    // Designated verifier confirms delivery on-chain
    const confirmTs = env.now();
    expectOk(
      env.send([await ixConfirmDelivery(verifier.publicKey, escrow)], [verifier])
    );

    const state = decodeAccount<any>(program, "escrow", env.accountData(escrow));
    expect(state.deliveryConfirmed).to.eq(true);
    expect(BigInt(state.confirmedAt.toString())).to.eq(confirmTs);

    // Once delivery is confirmed, maker cannot hike price via update OR prematurely refund before expiry!
    expectErr(
      env.send(
        [
          await ixUpdate({
            maker: maker.publicKey,
            seed,
            newReceive: 999n * UNIT,
            newDuration: 3600,
            newVerifier: verifier.publicKey,
          }),
        ],
        [maker]
      ),
      "AlreadyConfirmed"
    );
    expectErr(
      env.send(
        [
          await ixRefund({
            caller: maker.publicKey,
            maker: maker.publicKey,
            mintA,
            seed,
          }),
        ],
        [maker]
      ),
      "DeliveryConfirmedActive"
    );

    // Taker settles cleanly without needing verifier signature again
    expectOk(
      env.send(
        [
          await ixTake({
            taker: taker.publicKey,
            maker: maker.publicKey,
            mintA,
            mintB,
            seed,
            expectedReceive: receive,
          }),
        ],
        [taker]
      )
    );
    expect(env.tokenBalance(ataOf(mintA, taker.publicKey))).to.eq(deposit);
    expect(env.tokenBalance(ataOf(mintB, maker.publicKey))).to.eq(receive);
  });

  it("conditional timed escrow (Mode B — inline verifier co-signature & post-expiry liveness): supports single-tx co-sign AND recovery after expiry", async () => {
    const { env, maker, taker, verifier, crank, mintA, mintB, makerAtaA } = setup();

    // Case 1: Single-transaction inline co-signature by verifier on `take`
    const seed1 = 7n;
    expectOk(
      env.send(
        [
          await ixMake({
            maker: maker.publicKey,
            mintA,
            mintB,
            seed: seed1,
            deposit: 100n * UNIT,
            receive: 120n * UNIT,
            duration: 1800,
            verifier: verifier.publicKey,
          }),
        ],
        [maker]
      )
    );

    // Wrong co-signer rejected
    expectErr(
      env.send(
        [
          await ixTake({
            taker: taker.publicKey,
            maker: maker.publicKey,
            mintA,
            mintB,
            seed: seed1,
            expectedReceive: 120n * UNIT,
            verifier: crank.publicKey,
          }),
        ],
        [taker, crank]
      ),
      "UnauthorizedVerifier"
    );

    // Valid co-signature settles atomically without prior `confirm_delivery`
    expectOk(
      env.send(
        [
          await ixTake({
            taker: taker.publicKey,
            maker: maker.publicKey,
            mintA,
            mintB,
            seed: seed1,
            expectedReceive: 120n * UNIT,
            verifier: verifier.publicKey,
          }),
        ],
        [taker, verifier]
      )
    );
    expect(env.tokenBalance(ataOf(mintA, taker.publicKey))).to.eq(100n * UNIT);

    // Case 2: Liveness guarantee — if delivery is confirmed, but taker disappears past expiry, maker can recover funds!
    const seed2 = 8n;
    expectOk(
      env.send(
        [
          await ixMake({
            maker: maker.publicKey,
            mintA,
            mintB,
            seed: seed2,
            deposit: 100n * UNIT,
            receive: 100n * UNIT,
            duration: 1800,
            verifier: verifier.publicKey,
          }),
        ],
        [maker]
      )
    );
    const { escrow: escrow2 } = findEscrow(maker.publicKey, seed2);
    expectOk(
      env.send([await ixConfirmDelivery(verifier.publicKey, escrow2)], [verifier])
    );

    // Warp past 1800s expiry -> maker recovers funds cleanly
    env.warp(1801);
    expectOk(
      env.send(
        [
          await ixRefund({
            caller: maker.publicKey,
            maker: maker.publicKey,
            mintA,
            seed: seed2,
          }),
        ],
        [maker]
      )
    );
    // Started with 1000, sold 100 in seed1, recovered 100 in seed2 => 900 Mint A
    expect(env.tokenBalance(makerAtaA)).to.eq(900n * UNIT);
  });
});
