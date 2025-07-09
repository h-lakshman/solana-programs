import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Clmm } from "../target/types/clmm";
import { createMint } from "@solana/spl-token";
import { assert } from "chai";
import { BASE_SQRT_PRICE_X64 } from "./utils";
import { sqrt } from "bn-sqrt";

describe("clmm", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.clmm as Program<Clmm>;
  const provider = anchor.getProvider();
  const poolCreator = provider.wallet;
  let poolAuthorityPda: anchor.web3.PublicKey;
  let poolAuthorityBump: Number;
  let tokenAMint: anchor.web3.PublicKey;
  let tokenBMint: anchor.web3.PublicKey;
  let lpMintPda: anchor.web3.PublicKey;
  let lpMintBump: Number;
  let vaultAPda: anchor.web3.PublicKey;
  let vaultABump: Number;
  let vaultBPda: anchor.web3.PublicKey;
  let vaultBBump: Number;
  let poolPda: anchor.web3.PublicKey;
  let poolBump: Number;
  let lpCreatorTokenAccount: anchor.web3.PublicKey;

  before(async () => {
    // Create token A mint,assuming tokenA as base as Sol
    tokenAMint = await createMint(
      provider.connection,
      poolCreator.payer,
      poolCreator.publicKey,
      poolCreator.publicKey,
      6
    );

    // Create token B mint,assuming tokenB as base as USDC
    tokenBMint = await createMint(
      provider.connection,
      poolCreator.payer,
      poolCreator.publicKey,
      poolCreator.publicKey,
      6
    );

    [poolAuthorityPda, poolAuthorityBump] =
      anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("authority"), tokenAMint.toBytes(), tokenBMint.toBytes()],
        program.programId
      );

    [lpMintPda, lpMintBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("lp_mint"), tokenAMint.toBytes(), tokenBMint.toBytes()],
      program.programId
    );

    [vaultAPda, vaultABump] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("vault_token"),
        tokenAMint.toBytes(),
        tokenBMint.toBytes(),
        Buffer.from("A"),
      ],
      program.programId
    );

    [vaultBPda, vaultBBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("vault_token"),
        tokenAMint.toBytes(),
        tokenBMint.toBytes(),
        Buffer.from("B"),
      ],
      program.programId
    );

    [poolPda, poolBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("pool"), tokenAMint.toBytes(), tokenBMint.toBytes()],
      program.programId
    );
  });

  describe("Initialize Pool", () => {
    it("should initialize a pool with correct parameters", async () => {
      // Assuming current price of 1 SOL = 150 USDC
      const currentPrice = new anchor.BN(150);
      const currentPriceSqrtX64 = sqrt(currentPrice)
        .mul(BASE_SQRT_PRICE_X64)
        .toString();
      const tx = await program.methods
        .initializePool(currentPrice)
        .accounts({
          initializer: poolCreator.publicKey,
          tokenAMint: tokenAMint,
          tokenBMint: tokenBMint,
        })
        .rpc();

      console.log("Initialize pool transaction signature:", tx);

      const poolAccount = await program.account.pool.fetch(poolPda);

      assert.equal(poolAccount.mintA.toString(), tokenAMint.toString());
      assert.equal(poolAccount.mintB.toString(), tokenBMint.toString());
      assert.equal(
        poolAccount.poolAuthority.toString(),
        poolAuthorityPda.toString()
      );
      assert.equal(poolAccount.vaultA.toString(), vaultAPda.toString());
      assert.equal(poolAccount.vaultB.toString(), vaultBPda.toString());
      assert.equal(poolAccount.lpMint.toString(), lpMintPda.toString());

      assert.equal(poolAccount.activeLiquidity.toString(), "0");
      assert.equal(poolAccount.totalLpIssued.toString(), "0");

      assert.equal(poolAccount.sqrtPriceX64.toString(), currentPriceSqrtX64);
      assert.equal(poolAccount.currentTick, 0);

      // Verify vault accounts were created
      const vaultAInfo = await provider.connection.getAccountInfo(vaultAPda);
      const vaultBInfo = await provider.connection.getAccountInfo(vaultBPda);

      assert.isNotNull(vaultAInfo);
      assert.isNotNull(vaultBInfo);

      const lpMintInfo = await provider.connection.getAccountInfo(lpMintPda);
      assert.isNotNull(lpMintInfo);
    });

    it("should fail to initialize pool with same token mints", async () => {
      try {
        await program.methods
          .initializePool(new anchor.BN(100))
          .accounts({
            initializer: poolCreator.publicKey,
            tokenAMint: tokenAMint,
            tokenBMint: tokenAMint, // Same mint for both tokens
          })
          .rpc();

        // Should not reach here
        assert.fail("Expected transaction to fail");
      } catch (error) {
        assert.include(error.message, "Token A and Token B must be different");
      }
    });

    it("should fail to initialize pool twice", async () => {
      try {
        await program.methods
          .initializePool(new anchor.BN(200))
          .accounts({
            initializer: poolCreator.publicKey,
            tokenAMint: tokenAMint,
            tokenBMint: tokenBMint,
          })
          .rpc();

        // Should not reach here
        assert.fail("Expected transaction to fail");
      } catch (error) {
        // Pool already exists, so account creation should fail
        assert.include(error.message, "Simulation failed");
      }
    });
  });
});
