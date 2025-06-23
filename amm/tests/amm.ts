import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Amm } from "../target/types/amm";
import {
  createInitializeMintInstruction,
  TOKEN_PROGRAM_ID,
  MINT_SIZE,
  getMint,
  getAccount,
} from "@solana/spl-token";
import { assert } from "chai";

describe("Automated Market Maker", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider();
  const program = anchor.workspace.amm as Program<Amm>;

  const initializer = provider.wallet.publicKey;
  const tokenAMint = anchor.web3.Keypair.generate();
  const tokenBMint = anchor.web3.Keypair.generate();
  const tokensAuthority = anchor.web3.Keypair.generate();
  let ammPoolPda: anchor.web3.PublicKey;
  let ammBump: number;
  let vaultA: anchor.web3.PublicKey;
  let vaultABump: number;
  let vaultB: anchor.web3.PublicKey;
  let vaultBBump: number;
  let lpMint: anchor.web3.PublicKey;
  let lpBump: number;
  let authority: anchor.web3.PublicKey;
  let authorityBump: number;

  before(async () => {
    const mintRent =
      await provider.connection.getMinimumBalanceForRentExemption(MINT_SIZE);

    const createMintAIx = anchor.web3.SystemProgram.createAccount({
      fromPubkey: initializer,
      newAccountPubkey: tokenAMint.publicKey,
      lamports: mintRent,
      space: MINT_SIZE,
      programId: TOKEN_PROGRAM_ID,
    });

    const createMintBIx = anchor.web3.SystemProgram.createAccount({
      fromPubkey: initializer,
      newAccountPubkey: tokenBMint.publicKey,
      lamports: mintRent,
      space: MINT_SIZE,
      programId: TOKEN_PROGRAM_ID,
    });

    const initializeMintAIx = createInitializeMintInstruction(
      tokenAMint.publicKey,
      6,
      tokensAuthority.publicKey,
      null,
      TOKEN_PROGRAM_ID
    );

    const initializeMintBIx = createInitializeMintInstruction(
      tokenBMint.publicKey,
      6,
      tokensAuthority.publicKey,
      null,
      TOKEN_PROGRAM_ID
    );

    const tx = new anchor.web3.Transaction().add(
      createMintAIx,
      createMintBIx,
      initializeMintAIx,
      initializeMintBIx
    );

    await provider.sendAndConfirm(tx, [tokenAMint, tokenBMint]);

    [ammPoolPda, ammBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("pool"),
        tokenAMint.publicKey.toBytes(),
        tokenBMint.publicKey.toBytes(),
      ],
      program.programId
    );

    [authority, authorityBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("authority"),
        tokenAMint.publicKey.toBytes(),
        tokenBMint.publicKey.toBytes(),
      ],
      program.programId
    );

    [vaultA, vaultABump] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("vault_token"),
        tokenAMint.publicKey.toBytes(),
        tokenBMint.publicKey.toBytes(),
        Buffer.from("A"),
      ],
      program.programId
    );

    [vaultB, vaultBBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("vault_token"),
        tokenAMint.publicKey.toBytes(),
        tokenBMint.publicKey.toBytes(),
        Buffer.from("B"),
      ],
      program.programId
    );

    [lpMint, lpBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("lp_mint"),
        tokenAMint.publicKey.toBytes(),
        tokenBMint.publicKey.toBytes(),
      ],
      program.programId
    );
  });

  describe("Initialize Pool", () => {
    it("Should initialize the pool successfully", async () => {
      const tx = await program.methods
        .initializePool()
        .accounts({
          initializer: initializer,
          tokenAMint: tokenAMint.publicKey,
          tokenBMint: tokenBMint.publicKey,
        })
        .rpc();

      const pool = await program.account.ammPool.fetch(ammPoolPda);
      assert.strictEqual(
        pool.mintA.toBase58(),
        tokenAMint.publicKey.toBase58(),
        "Pool mint A should match"
      );
      assert.strictEqual(
        pool.mintB.toBase58(),
        tokenBMint.publicKey.toBase58(),
        "Pool mint B should match"
      );
      assert.strictEqual(
        pool.vaultA.toBase58(),
        vaultA.toBase58(),
        "Pool vault A should match"
      );
      assert.strictEqual(
        pool.vaultB.toBase58(),
        vaultB.toBase58(),
        "Pool vault B should match"
      );
      assert.strictEqual(
        pool.lpMint.toBase58(),
        lpMint.toBase58(),
        "Pool LP mint should match"
      );
      assert.strictEqual(
        pool.totalLpIssued.toString(),
        "0",
        "Initial LP issued should be 0"
      );
      assert.strictEqual(
        pool.bump.toString(),
        ammBump.toString(),
        "Pool bump should match"
      );

      const vaultAAccount = await getAccount(provider.connection, vaultA);
      assert.strictEqual(
        vaultAAccount.mint.toBase58(),
        tokenAMint.publicKey.toBase58(),
        "Vault A should have correct mint"
      );
      assert.strictEqual(
        vaultAAccount.owner.toBase58(),
        authority.toBase58(),
        "Vault A should be owned by authority"
      );
      assert.strictEqual(
        vaultAAccount.amount.toString(),
        "0",
        "Vault A should start with 0 tokens"
      );

      const vaultBAccount = await getAccount(provider.connection, vaultB);
      assert.strictEqual(
        vaultBAccount.mint.toBase58(),
        tokenBMint.publicKey.toBase58(),
        "Vault B should have correct mint"
      );
      assert.strictEqual(
        vaultBAccount.owner.toBase58(),
        authority.toBase58(),
        "Vault B should be owned by authority"
      );
      assert.strictEqual(
        vaultBAccount.amount.toString(),
        "0",
        "Vault B should start with 0 tokens"
      );

      const lpMintAccount = await getMint(provider.connection, lpMint);
      assert.strictEqual(
        lpMintAccount.mintAuthority?.toBase58(),
        authority.toBase58(),
        "LP mint authority should be the AMM authority"
      );
      assert.strictEqual(
        lpMintAccount.freezeAuthority?.toBase58(),
        authority.toBase58(),
        "LP freeze authority should be the AMM authority"
      );
      assert.strictEqual(
        lpMintAccount.decimals,
        6,
        "LP mint should have 6 decimals"
      );
      assert.strictEqual(
        lpMintAccount.supply.toString(),
        "0",
        "LP mint supply should start at 0"
      );
    });

    it("Should fail when trying to initialize with the same token", async () => {
      try {
        await program.methods
          .initializePool()
          .accounts({
            initializer: initializer,
            tokenAMint: tokenAMint.publicKey,
            tokenBMint: tokenAMint.publicKey,
          })
          .rpc();

        assert.fail("Should have failed with same token error");
      } catch (error) {
        assert.include(
          error.toString(),
          "SameTokenMint",
          "Should fail with SameTokenMint error"
        );
      }
    });

    it("Should fail when trying to initialize pool twice", async () => {
      try {
        await program.methods
          .initializePool()
          .accounts({
            initializer: initializer,
            tokenAMint: tokenAMint.publicKey,
            tokenBMint: tokenBMint.publicKey,
          })
          .rpc();

        assert.fail("Should have failed trying to initialize twice");
      } catch (error) {
        assert.include(
          error.toString(),
          "already in use",
          "Should fail because accounts already exist"
        );
      }
    });
  });
});
