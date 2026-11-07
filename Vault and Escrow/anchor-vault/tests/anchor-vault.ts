import fs from "fs";
import path from "path";
import { expect } from "chai";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import type { AnchorVault } from "../target/types/anchor_vault";
import {
  BN,
  decodeAccount,
  Env,
  expectErr,
  expectOk,
  LAMPORTS_PER_SOL,
  loadProgram,
  TX_FEE,
} from "./utils";

const idl = JSON.parse(
  fs.readFileSync(path.resolve(__dirname, "../target/idl/anchor_vault.json"), "utf8")
);
const SO_PATH = path.resolve(__dirname, "../target/deploy/anchor_vault.so");
const ONE_SOL = BigInt(LAMPORTS_PER_SOL);

describe("anchor-vault (TypeScript + LiteSVM)", () => {
  const program = loadProgram<AnchorVault>(idl as AnchorVault);
  const programId = program.programId;

  function findPdas(user: PublicKey) {
    const [vaultState, stateBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("state"), user.toBuffer()],
      programId
    );
    const [vault, vaultBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), user.toBuffer()],
      programId
    );
    return { vaultState, stateBump, vault, vaultBump };
  }

  function setup() {
    const env = new Env();
    env.addProgram(programId, SO_PATH);
    const user = env.wallet(10);
    const pdas = findPdas(user.publicKey);
    return { env, user, ...pdas };
  }

  async function ixInitialize(user: PublicKey) {
    const { vaultState, vault } = findPdas(user);
    return program.methods
      .initialize()
      .accountsPartial({
        user,
        vaultState,
        vault,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
  }

  async function ixDeposit(user: PublicKey, amount: bigint) {
    const { vaultState, vault } = findPdas(user);
    return program.methods
      .deposit(new BN(amount.toString()))
      .accountsPartial({
        user,
        vaultState,
        vault,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
  }

  async function ixWithdraw(
    user: PublicKey,
    amount: bigint,
    override?: { vaultState: PublicKey; vault: PublicKey }
  ) {
    const { vaultState, vault } = override ?? findPdas(user);
    return program.methods
      .withdraw(new BN(amount.toString()))
      .accountsPartial({
        user,
        vaultState,
        vault,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
  }

  async function ixClose(user: PublicKey) {
    const { vaultState, vault } = findPdas(user);
    return program.methods
      .close()
      .accountsPartial({
        user,
        vaultState,
        vault,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
  }

  it("initializes VaultState PDA and funds a rent-exempt Vault PDA", async () => {
    const { env, user, vaultState, stateBump, vault, vaultBump } = setup();

    expectOk(env.send([await ixInitialize(user.publicKey)], [user]));

    expect(env.owner(vaultState)?.toBase58()).to.eq(programId.toBase58());
    const state = decodeAccount<{ vaultBump: number; stateBump: number }>(
      program,
      "vaultState",
      env.accountData(vaultState)
    );
    expect(state.stateBump).to.eq(stateBump);
    expect(state.vaultBump).to.eq(vaultBump);

    const rentExempt = env.rentExempt(0);
    expect(env.balance(vault)).to.eq(rentExempt);
  });

  it("rejects duplicate initialization for the same user", async () => {
    const { env, user } = setup();
    expectOk(env.send([await ixInitialize(user.publicKey)], [user]));
    expectErr(env.send([await ixInitialize(user.publicKey)], [user]));
  });

  it("deposits SOL across multiple transactions and tracks balances accurately", async () => {
    const { env, user, vault } = setup();
    expectOk(env.send([await ixInitialize(user.publicKey)], [user]));

    const vaultBefore = env.balance(vault);
    const userBefore = env.balance(user.publicKey);

    expectOk(env.send([await ixDeposit(user.publicKey, ONE_SOL)], [user]));
    expectOk(env.send([await ixDeposit(user.publicKey, 2n * ONE_SOL)], [user]));

    expect(env.balance(vault)).to.eq(vaultBefore + 3n * ONE_SOL);
    expect(env.balance(user.publicKey)).to.eq(
      userBefore - 3n * ONE_SOL - 2n * TX_FEE
    );
  });

  it("rejects zero-amount deposit and deposits before initialization", async () => {
    const { env, user } = setup();

    // Uninitialized vault fails with AccountNotInitialized
    expectErr(
      env.send([await ixDeposit(user.publicKey, ONE_SOL)], [user]),
      "AccountNotInitialized"
    );

    expectOk(env.send([await ixInitialize(user.publicKey)], [user]));
    expectErr(
      env.send([await ixDeposit(user.publicKey, 0n)], [user]),
      "InvalidAmount"
    );
  });

  it("withdraws partial and full deposits while preserving the rent-exempt reserve", async () => {
    const { env, user, vault } = setup();
    expectOk(env.send([await ixInitialize(user.publicKey)], [user]));
    expectOk(env.send([await ixDeposit(user.publicKey, 2n * ONE_SOL)], [user]));

    const userBefore = env.balance(user.publicKey);
    expectOk(
      env.send([await ixWithdraw(user.publicKey, ONE_SOL / 2n)], [user])
    );
    expect(env.balance(user.publicKey)).to.eq(
      userBefore + ONE_SOL / 2n - TX_FEE
    );

    // Withdraw remaining 1.5 SOL -> vault retains exact rent-exempt reserve
    expectOk(
      env.send([await ixWithdraw(user.publicKey, (3n * ONE_SOL) / 2n)], [user])
    );
    expect(env.balance(vault)).to.eq(env.rentExempt(0));
  });

  it("rejects zero-amount withdrawals and withdrawals exceeding available funds", async () => {
    const { env, user } = setup();
    expectOk(env.send([await ixInitialize(user.publicKey)], [user]));
    expectOk(env.send([await ixDeposit(user.publicKey, ONE_SOL)], [user]));

    expectErr(
      env.send([await ixWithdraw(user.publicKey, 0n)], [user]),
      "InvalidAmount"
    );
    // 1 lamport above deposit attempts to dip into the rent reserve
    expectErr(
      env.send([await ixWithdraw(user.publicKey, ONE_SOL + 1n)], [user]),
      "InsufficientFunds"
    );
  });

  it("prevents an attacker from withdrawing from another user's vault", async () => {
    const { env, user, vaultState, vault } = setup();
    expectOk(env.send([await ixInitialize(user.publicKey)], [user]));
    expectOk(env.send([await ixDeposit(user.publicKey, ONE_SOL)], [user]));

    const attacker: Keypair = env.wallet(2);
    const badIx = await ixWithdraw(attacker.publicKey, ONE_SOL, {
      vaultState,
      vault,
    });
    expectErr(env.send([badIx], [attacker]), "ConstraintSeeds");
    expect(env.balance(vault)).to.eq(ONE_SOL + env.rentExempt(0));
  });

  it("closes the vault, drains all lamports + rent to user, and allows re-initialization", async () => {
    const { env, user, vaultState, vault } = setup();
    const initialUserBalance = env.balance(user.publicKey);

    expectOk(env.send([await ixInitialize(user.publicKey)], [user]));
    expectOk(env.send([await ixDeposit(user.publicKey, 3n * ONE_SOL)], [user]));
    expectOk(env.send([await ixWithdraw(user.publicKey, ONE_SOL)], [user]));
    expectOk(env.send([await ixClose(user.publicKey)], [user]));

    expect(env.exists(vaultState)).to.eq(false);
    expect(env.balance(vault)).to.eq(0n);
    // Every lamport of deposit + rent came back; only 4 tx fees were paid.
    expect(env.balance(user.publicKey)).to.eq(initialUserBalance - 4n * TX_FEE);

    // Can re-open vault cleanly after closing
    expectOk(env.send([await ixInitialize(user.publicKey)], [user]));
    expect(env.exists(vaultState)).to.eq(true);
  });
});
