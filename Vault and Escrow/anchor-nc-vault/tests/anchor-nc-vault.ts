import fs from "fs";
import path from "path";
import { expect } from "chai";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { createTransferInstruction } from "@solana/spl-token";
import type { AnchorNcVault } from "../target/types/anchor_nc_vault";
import type { MockMarket } from "../target/types/mock_market";
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

const vaultIdl = JSON.parse(
  fs.readFileSync(path.resolve(__dirname, "../target/idl/anchor_nc_vault.json"), "utf8")
);
const marketIdl = JSON.parse(
  fs.readFileSync(path.resolve(__dirname, "../target/idl/mock_market.json"), "utf8")
);
const VAULT_SO = path.resolve(__dirname, "../target/deploy/anchor_nc_vault.so");
const MARKET_SO = path.resolve(__dirname, "../target/deploy/mock_market.so");
const DECIMALS = 6;
const USDC = 1_000_000n; // 1.000000 USDC

describe("anchor-nc-vault: Non-Custodial Share Vault with In-Kind Market Redemption (TypeScript + LiteSVM)", () => {
  const vaultProgram = loadProgram<AnchorNcVault>(vaultIdl as AnchorNcVault);
  const marketProgram = loadProgram<MockMarket>(marketIdl as MockMarket);

  function findMarketPdas(authority: PublicKey, baseMint: PublicKey) {
    const [market] = PublicKey.findProgramAddressSync(
      [Buffer.from("market"), authority.toBuffer(), baseMint.toBuffer()],
      marketProgram.programId
    );
    const [positionMint] = PublicKey.findProgramAddressSync(
      [Buffer.from("position"), market.toBuffer()],
      marketProgram.programId
    );
    const marketLiquidity = ataOf(baseMint, market);
    return { market, positionMint, marketLiquidity };
  }

  function findVaultPdas(
    manager: PublicKey,
    market: PublicKey,
    baseMint: PublicKey,
    positionMint: PublicKey
  ) {
    const [vault] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), manager.toBuffer(), market.toBuffer()],
      vaultProgram.programId
    );
    const [shareMint] = PublicKey.findProgramAddressSync(
      [Buffer.from("shares"), vault.toBuffer()],
      vaultProgram.programId
    );
    const idle = ataOf(baseMint, vault);
    const vaultPosition = ataOf(positionMint, vault);
    return { vault, shareMint, idle, vaultPosition };
  }

  async function setup() {
    const env = new Env();
    env.addProgram(marketProgram.programId, MARKET_SO);
    env.addProgram(vaultProgram.programId, VAULT_SO);

    const mintAuthority = env.wallet(10);
    const marketAuthority = env.wallet(10);
    const manager = env.wallet(10);
    const alice = env.wallet(10);
    const bob = env.wallet(10);

    const baseMint = env.createMint(mintAuthority, mintAuthority.publicKey, DECIMALS);

    // Fund users and marketAuthority (for borrower/repay simulation)
    env.mintTo(mintAuthority, baseMint, alice.publicKey, 5_000n * USDC);
    env.mintTo(mintAuthority, baseMint, bob.publicKey, 5_000n * USDC);
    env.mintTo(mintAuthority, baseMint, marketAuthority.publicKey, 1_000n * USDC);

    const { market, positionMint, marketLiquidity } = findMarketPdas(
      marketAuthority.publicKey,
      baseMint
    );

    // 1. Initialize underlying lending market
    const initMarketIx = await marketProgram.methods
      .initialize()
      .accountsPartial({
        authority: marketAuthority.publicKey,
        baseMint,
        market,
        positionMint,
        liquidity: marketLiquidity,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
    expectOk(env.send([initMarketIx], [marketAuthority]));

    // 2. Initialize non-custodial vault bound to `market`
    const { vault, shareMint, idle, vaultPosition } = findVaultPdas(
      manager.publicKey,
      market,
      baseMint,
      positionMint
    );
    const initVaultIx = await vaultProgram.methods
      .initialize()
      .accountsPartial({
        manager: manager.publicKey,
        market,
        baseMint,
        positionMint,
        vault,
        shareMint,
        idle,
        vaultPosition,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
    expectOk(env.send([initVaultIx], [manager]));

    return {
      env,
      mintAuthority,
      marketAuthority,
      manager,
      alice,
      bob,
      baseMint,
      market,
      positionMint,
      marketLiquidity,
      vault,
      shareMint,
      idle,
      vaultPosition,
    };
  }

  it("initializes market & vault and mints pro-rata shares on deposit with slippage protection", async () => {
    const c = await setup();

    const state = decodeAccount<any>(
      vaultProgram,
      "vault",
      c.env.accountData(c.vault)
    );
    expect(state.manager.toBase58()).to.eq(c.manager.publicKey.toBase58());
    expect(state.market.toBase58()).to.eq(c.market.toBase58());
    expect(state.shareMint.toBase58()).to.eq(c.shareMint.toBase58());

    // Zero deposit fails
    const zeroIx = await vaultProgram.methods
      .deposit(new BN(0), new BN(0))
      .accountsPartial({
        user: c.alice.publicKey,
        vault: c.vault,
        baseMint: c.baseMint,
        positionMint: c.positionMint,
        shareMint: c.shareMint,
        userBase: ataOf(c.baseMint, c.alice.publicKey),
        userShares: ataOf(c.shareMint, c.alice.publicKey),
        idle: c.idle,
        vaultPosition: c.vaultPosition,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
    expectErr(c.env.send([zeroIx], [c.alice]), "InvalidAmount");

    // Unrealistic minShares fails with SlippageExceeded
    const badSlippageIx = await vaultProgram.methods
      .deposit(new BN((1_000n * USDC).toString()), new BN((1_001n * USDC).toString()))
      .accountsPartial({
        user: c.alice.publicKey,
        vault: c.vault,
        baseMint: c.baseMint,
        positionMint: c.positionMint,
        shareMint: c.shareMint,
        userBase: ataOf(c.baseMint, c.alice.publicKey),
        userShares: ataOf(c.shareMint, c.alice.publicKey),
        idle: c.idle,
        vaultPosition: c.vaultPosition,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
    expectErr(c.env.send([badSlippageIx], [c.alice]), "SlippageExceeded");

    // Valid deposit: Alice deposits 1,000 USDC -> gets 1,000 shares
    const depIx = await vaultProgram.methods
      .deposit(new BN((1_000n * USDC).toString()), new BN((1_000n * USDC).toString()))
      .accountsPartial({
        user: c.alice.publicKey,
        vault: c.vault,
        baseMint: c.baseMint,
        positionMint: c.positionMint,
        shareMint: c.shareMint,
        userBase: ataOf(c.baseMint, c.alice.publicKey),
        userShares: ataOf(c.shareMint, c.alice.publicKey),
        idle: c.idle,
        vaultPosition: c.vaultPosition,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
    expectOk(c.env.send([depIx], [c.alice]));

    expect(c.env.tokenBalance(c.idle)).to.eq(1_000n * USDC);
    expect(c.env.tokenBalance(ataOf(c.shareMint, c.alice.publicKey))).to.eq(
      1_000n * USDC
    );
  });

  it("allows manager to rebalance (allocate/deallocate) strictly within vault-owned accounts and supports liquid withdraw", async () => {
    const c = await setup();

    // Alice & Bob each deposit 1,000 USDC (total 2,000 USDC)
    for (const user of [c.alice, c.bob]) {
      const ix = await vaultProgram.methods
        .deposit(new BN((1_000n * USDC).toString()), new BN(0))
        .accountsPartial({
          user: user.publicKey,
          vault: c.vault,
          baseMint: c.baseMint,
          positionMint: c.positionMint,
          shareMint: c.shareMint,
          userBase: ataOf(c.baseMint, user.publicKey),
          userShares: ataOf(c.shareMint, user.publicKey),
          idle: c.idle,
          vaultPosition: c.vaultPosition,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .instruction();
      expectOk(c.env.send([ix], [user]));
    }

    // Non-manager cannot call allocate
    const unauthAllocIx = await vaultProgram.methods
      .allocate(new BN((500n * USDC).toString()))
      .accountsPartial({
        manager: c.alice.publicKey,
        vault: c.vault,
        idle: c.idle,
        vaultPosition: c.vaultPosition,
        market: c.market,
        baseMint: c.baseMint,
        positionMint: c.positionMint,
        marketLiquidity: c.marketLiquidity,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        marketProgram: marketProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();
    expectErr(c.env.send([unauthAllocIx], [c.alice]));

    // Manager allocates 1,500 USDC into the market -> idle = 500, vaultPosition = 1,500
    const allocIx = await vaultProgram.methods
      .allocate(new BN((1_500n * USDC).toString()))
      .accountsPartial({
        manager: c.manager.publicKey,
        vault: c.vault,
        idle: c.idle,
        vaultPosition: c.vaultPosition,
        market: c.market,
        baseMint: c.baseMint,
        positionMint: c.positionMint,
        marketLiquidity: c.marketLiquidity,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        marketProgram: marketProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();
    expectOk(c.env.send([allocIx], [c.manager]));
    expect(c.env.tokenBalance(c.idle)).to.eq(500n * USDC);
    expect(c.env.tokenBalance(c.vaultPosition)).to.eq(1_500n * USDC);
    // Manager wallet holds zero base/position tokens (non-custodial!)
    expect(c.env.tokenBalance(ataOf(c.baseMint, c.manager.publicKey))).to.eq(0n);
    expect(c.env.tokenBalance(ataOf(c.positionMint, c.manager.publicKey))).to.eq(0n);

    // Manager deallocates 300 USDC -> idle = 800, vaultPosition = 1,200
    const deallocIx = await vaultProgram.methods
      .deallocate(new BN((300n * USDC).toString()))
      .accountsPartial({
        manager: c.manager.publicKey,
        vault: c.vault,
        idle: c.idle,
        vaultPosition: c.vaultPosition,
        market: c.market,
        baseMint: c.baseMint,
        positionMint: c.positionMint,
        marketLiquidity: c.marketLiquidity,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        marketProgram: marketProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();
    expectOk(c.env.send([deallocIx], [c.manager]));
    expect(c.env.tokenBalance(c.idle)).to.eq(800n * USDC);
    expect(c.env.tokenBalance(c.vaultPosition)).to.eq(1_200n * USDC);

    // Alice withdraws 500 shares cleanly from idle liquidity
    const aliceBaseBefore = c.env.tokenBalance(ataOf(c.baseMint, c.alice.publicKey));
    const withdrawIx = await vaultProgram.methods
      .withdraw(new BN((500n * USDC).toString()), new BN((500n * USDC).toString()))
      .accountsPartial({
        user: c.alice.publicKey,
        vault: c.vault,
        baseMint: c.baseMint,
        positionMint: c.positionMint,
        shareMint: c.shareMint,
        userBase: ataOf(c.baseMint, c.alice.publicKey),
        userShares: ataOf(c.shareMint, c.alice.publicKey),
        idle: c.idle,
        vaultPosition: c.vaultPosition,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
    expectOk(c.env.send([withdrawIx], [c.alice]));
    expect(c.env.tokenBalance(ataOf(c.baseMint, c.alice.publicKey))).to.eq(
      aliceBaseBefore + 500n * USDC
    );
  });

  it("redeem_in_kind: lets users exit ANYTIME for underlying market positions even when BOTH vault and market have ZERO liquidity", async () => {
    const c = await setup();

    // 1. Alice and Bob each deposit 1,000 USDC -> 2,000 USDC in vault
    for (const user of [c.alice, c.bob]) {
      const ix = await vaultProgram.methods
        .deposit(new BN((1_000n * USDC).toString()), new BN(0))
        .accountsPartial({
          user: user.publicKey,
          vault: c.vault,
          baseMint: c.baseMint,
          positionMint: c.positionMint,
          shareMint: c.shareMint,
          userBase: ataOf(c.baseMint, user.publicKey),
          userShares: ataOf(c.shareMint, user.publicKey),
          idle: c.idle,
          vaultPosition: c.vaultPosition,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .instruction();
      expectOk(c.env.send([ix], [user]));
    }

    // 2. Manager allocates 100% (2,000 USDC) into mock_market -> vault idle = 0!
    const allocAllIx = await vaultProgram.methods
      .allocate(new BN((2_000n * USDC).toString()))
      .accountsPartial({
        manager: c.manager.publicKey,
        vault: c.vault,
        idle: c.idle,
        vaultPosition: c.vaultPosition,
        market: c.market,
        baseMint: c.baseMint,
        positionMint: c.positionMint,
        marketLiquidity: c.marketLiquidity,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        marketProgram: marketProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();
    expectOk(c.env.send([allocAllIx], [c.manager]));
    expect(c.env.tokenBalance(c.idle)).to.eq(0n);

    // 3. Borrowers drain 100% of market liquidity too (2,000 USDC) -> market liquidity = 0!
    const borrowAllIx = await marketProgram.methods
      .borrow(new BN((2_000n * USDC).toString()))
      .accountsPartial({
        authority: c.marketAuthority.publicKey,
        authorityBase: ataOf(c.baseMint, c.marketAuthority.publicKey),
        market: c.market,
        baseMint: c.baseMint,
        liquidity: c.marketLiquidity,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();
    expectOk(c.env.send([borrowAllIx], [c.marketAuthority]));
    expect(c.env.tokenBalance(c.marketLiquidity)).to.eq(0n);

    // 4. Liquid `withdraw` AND manager `deallocate` BOTH fail because there is 0 liquidity anywhere!
    const failedWithdrawIx = await vaultProgram.methods
      .withdraw(new BN((1_000n * USDC).toString()), new BN(0))
      .accountsPartial({
        user: c.alice.publicKey,
        vault: c.vault,
        baseMint: c.baseMint,
        positionMint: c.positionMint,
        shareMint: c.shareMint,
        userBase: ataOf(c.baseMint, c.alice.publicKey),
        userShares: ataOf(c.shareMint, c.alice.publicKey),
        idle: c.idle,
        vaultPosition: c.vaultPosition,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
    expectErr(
      c.env.send([failedWithdrawIx], [c.alice]),
      "InsufficientLiquidity"
    );

    const failedDeallocIx = await vaultProgram.methods
      .deallocate(new BN((1_000n * USDC).toString()))
      .accountsPartial({
        manager: c.manager.publicKey,
        vault: c.vault,
        idle: c.idle,
        vaultPosition: c.vaultPosition,
        market: c.market,
        baseMint: c.baseMint,
        positionMint: c.positionMint,
        marketLiquidity: c.marketLiquidity,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        marketProgram: marketProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();
    expectErr(
      c.env.send([failedDeallocIx], [c.manager]),
      "InsufficientLiquidity"
    );

    // 5. Alice exits IMMEDIATELY via `redeem_in_kind` -> receives 1,000 market position tokens!
    const alicePositionAta = ataOf(c.positionMint, c.alice.publicKey);
    const redeemIx = await vaultProgram.methods
      .redeemInKind(new BN((1_000n * USDC).toString()))
      .accountsPartial({
        user: c.alice.publicKey,
        vault: c.vault,
        baseMint: c.baseMint,
        positionMint: c.positionMint,
        shareMint: c.shareMint,
        userShares: ataOf(c.shareMint, c.alice.publicKey),
        userBase: ataOf(c.baseMint, c.alice.publicKey),
        userPosition: alicePositionAta,
        idle: c.idle,
        vaultPosition: c.vaultPosition,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
    expectOk(c.env.send([redeemIx], [c.alice]));

    // Alice's vault shares are burned and she now holds the underlying market position directly!
    expect(c.env.tokenBalance(ataOf(c.shareMint, c.alice.publicKey))).to.eq(0n);
    expect(c.env.tokenBalance(alicePositionAta)).to.eq(1_000n * USDC);
    expect(c.env.tokenBalance(c.vaultPosition)).to.eq(1_000n * USDC);
    expect(c.env.mintSupply(c.shareMint)).to.eq(1_000n * USDC);

    // 6. Later, borrowers repay the market -> Alice withdraws her position directly from `mock_market`!
    const repayIx = await marketProgram.methods
      .repay(new BN((2_000n * USDC).toString()))
      .accountsPartial({
        payer: c.marketAuthority.publicKey,
        payerBase: ataOf(c.baseMint, c.marketAuthority.publicKey),
        market: c.market,
        baseMint: c.baseMint,
        liquidity: c.marketLiquidity,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();
    expectOk(c.env.send([repayIx], [c.marketAuthority]));

    const aliceBaseBefore = c.env.tokenBalance(ataOf(c.baseMint, c.alice.publicKey));
    const directMarketWithdrawIx = await marketProgram.methods
      .withdraw(new BN((1_000n * USDC).toString()))
      .accountsPartial({
        user: c.alice.publicKey,
        userBase: ataOf(c.baseMint, c.alice.publicKey),
        userPosition: alicePositionAta,
        market: c.market,
        baseMint: c.baseMint,
        positionMint: c.positionMint,
        liquidity: c.marketLiquidity,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();
    expectOk(c.env.send([directMarketWithdrawIx], [c.alice]));

    expect(c.env.tokenBalance(alicePositionAta)).to.eq(0n);
    expect(c.env.tokenBalance(ataOf(c.baseMint, c.alice.publicKey))).to.eq(
      aliceBaseBefore + 1_000n * USDC
    );
  });

  it("redeem_in_kind: distributes exact pro-rata slice of BOTH idle liquidity and market positions (including yield)", async () => {
    const c = await setup();

    // Alice & Bob each deposit 1,000 USDC -> 2,000 shares total
    for (const user of [c.alice, c.bob]) {
      const ix = await vaultProgram.methods
        .deposit(new BN((1_000n * USDC).toString()), new BN(0))
        .accountsPartial({
          user: user.publicKey,
          vault: c.vault,
          baseMint: c.baseMint,
          positionMint: c.positionMint,
          shareMint: c.shareMint,
          userBase: ataOf(c.baseMint, user.publicKey),
          userShares: ataOf(c.shareMint, user.publicKey),
          idle: c.idle,
          vaultPosition: c.vaultPosition,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .instruction();
      expectOk(c.env.send([ix], [user]));
    }

    // Manager deploys 1,600 USDC to market (leaving 400 idle), plus 200 USDC yield accrues into idle -> 600 idle + 1,600 position = 2,200 NAV
    const allocIx = await vaultProgram.methods
      .allocate(new BN((1_600n * USDC).toString()))
      .accountsPartial({
        manager: c.manager.publicKey,
        vault: c.vault,
        idle: c.idle,
        vaultPosition: c.vaultPosition,
        market: c.market,
        baseMint: c.baseMint,
        positionMint: c.positionMint,
        marketLiquidity: c.marketLiquidity,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        marketProgram: marketProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();
    expectOk(c.env.send([allocIx], [c.manager]));

    // Simulate 200 USDC yield reward flowing into vault's idle account
    expectOk(
      c.env.send(
        [
          createTransferInstruction(
            ataOf(c.baseMint, c.marketAuthority.publicKey),
            c.idle,
            c.marketAuthority.publicKey,
            200n * USDC
          ),
        ],
        [c.marketAuthority]
      )
    );

    // Bob owns 50% of shares (1,000 / 2,000). Redeeming in-kind gives him 300 USDC + 800 mUSDC!
    const bobBaseBefore = c.env.tokenBalance(ataOf(c.baseMint, c.bob.publicKey));
    const redeemIx = await vaultProgram.methods
      .redeemInKind(new BN((1_000n * USDC).toString()))
      .accountsPartial({
        user: c.bob.publicKey,
        vault: c.vault,
        baseMint: c.baseMint,
        positionMint: c.positionMint,
        shareMint: c.shareMint,
        userShares: ataOf(c.shareMint, c.bob.publicKey),
        userBase: ataOf(c.baseMint, c.bob.publicKey),
        userPosition: ataOf(c.positionMint, c.bob.publicKey),
        idle: c.idle,
        vaultPosition: c.vaultPosition,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
    expectOk(c.env.send([redeemIx], [c.bob]));

    expect(c.env.tokenBalance(ataOf(c.baseMint, c.bob.publicKey))).to.eq(
      bobBaseBefore + 300n * USDC
    );
    expect(c.env.tokenBalance(ataOf(c.positionMint, c.bob.publicKey))).to.eq(
      800n * USDC
    );
  });
});
