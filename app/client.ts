import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";
import { randomBytes } from "crypto";
import * as x25519 from "@noble/curves/x25519";
import {
  RescueCipher,
  getArciumEnv,
  getMXEPublicKeyWithRetry,
  getCompDefAccAddress,
  getCompDefAccOffset,
  getComputationAccAddress,
  getClusterAccAddress,
  getMXEAccAddress,
  getMempoolAccAddress,
  getExecutingPoolAccAddress,
  awaitComputationFinalization,
  awaitEvent,
  deserializeLE,
} from "@arcium-hq/client";
import { PrivatePerps } from "../target/types/private_perps";

export interface OrderParams {
  size: bigint;
  limitPrice: bigint;
  isShort: boolean;
  leverage: number;
}

export interface PriceFeed {
  markPrice: bigint;
  fundingRateBps: bigint;
  timestamp: bigint;
}

export interface PnlResult {
  pnlUsdCents: bigint;
  isProfit: boolean;
}

export class PrivatePerpsClient {
  private program: Program<PrivatePerps>;
  private provider: anchor.AnchorProvider;
  private arciumEnv: ReturnType<typeof getArciumEnv>;

  constructor(program: Program<PrivatePerps>, provider: anchor.AnchorProvider) {
    this.program = program;
    this.provider = provider;
    this.arciumEnv = getArciumEnv();
  }

  async initCompDefs(payer: Keypair): Promise<void> {
    console.log("Initialising computation definitions...");
    await this.program.methods
      .initOpenPositionCompDef()
      .accounts({ payer: payer.publicKey })
      .signers([payer])
      .rpc({ commitment: "confirmed" });
    await this.program.methods
      .initLiquidationCheckCompDef()
      .accounts({ payer: payer.publicKey })
      .signers([payer])
      .rpc({ commitment: "confirmed" });
    await this.program.methods
      .initClosePositionCompDef()
      .accounts({ payer: payer.publicKey })
      .signers([payer])
      .rpc({ commitment: "confirmed" });
    console.log("Computation definitions initialised.");
  }

  async openPosition(
    trader: Keypair,
    order: OrderParams
  ): Promise<{ sharedSecret: Uint8Array; computationOffset: BN }> {
    const { cipher, publicKey, sharedSecret } = await this._buildCipher();
    const nonce = randomBytes(16);
    const plaintext: bigint[] = [
      order.size,
      order.limitPrice,
      BigInt(order.isShort ? 1 : 0),
      BigInt(order.leverage),
    ];
    const ciphertexts = cipher.encrypt(plaintext, nonce);
    const computationOffset = new BN(randomBytes(8), "hex");

    const sig = await this.program.methods
      .openPosition(
        computationOffset,
        Array.from(ciphertexts[0]),
        Array.from(ciphertexts[1]),
        Array.from(ciphertexts[2]),
        Array.from(ciphertexts[3]),
        Array.from(publicKey),
        new BN(deserializeLE(nonce).toString())
      )
      .accountsPartial({
        trader: trader.publicKey,
        computationAccount: getComputationAccAddress(
          this.arciumEnv.arciumClusterOffset, computationOffset),
        clusterAccount: getClusterAccAddress(this.arciumEnv.arciumClusterOffset),
        mxeAccount: getMXEAccAddress(this.program.programId),
        mempoolAccount: getMempoolAccAddress(this.arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(this.arciumEnv.arciumClusterOffset),
        compDefAccount: getCompDefAccAddress(
          this.program.programId,
          Buffer.from(getCompDefAccOffset("open_position")).readUInt32LE()),
      })
      .signers([trader])
      .rpc({ commitment: "confirmed" });

    console.log("open_position queued:", sig);
    await awaitComputationFinalization(
      this.provider, computationOffset, this.program.programId, "confirmed");
    console.log("Position opened successfully.");
    return { sharedSecret, computationOffset };
  }

  async liquidationCheck(
    keeper: Keypair,
    traderPubkey: PublicKey,
    feed: PriceFeed
  ): Promise<{ encResult: Uint8Array; nonce: Uint8Array }> {
    const { cipher, publicKey } = await this._buildCipher();
    const nonce = randomBytes(16);
    const feedCiphertexts = cipher.encrypt(
      [feed.markPrice, feed.fundingRateBps, feed.timestamp], nonce);
    const computationOffset = new BN(randomBytes(8), "hex");
    const liqCheckEventPromise = awaitEvent("liquidationCheckEvent");

    const sig = await this.program.methods
      .liquidationCheck(
        computationOffset,
        Array.from(feedCiphertexts[0]),
        Array.from(feedCiphertexts[1]),
        Array.from(feedCiphertexts[2]),
        Array.from(publicKey),
        new BN(deserializeLE(nonce).toString())
      )
      .accountsPartial({
        keeper: keeper.publicKey,
        trader: traderPubkey,
        computationAccount: getComputationAccAddress(
          this.arciumEnv.arciumClusterOffset, computationOffset),
        clusterAccount: getClusterAccAddress(this.arciumEnv.arciumClusterOffset),
        mxeAccount: getMXEAccAddress(this.program.programId),
        mempoolAccount: getMempoolAccAddress(this.arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(this.arciumEnv.arciumClusterOffset),
        compDefAccount: getCompDefAccAddress(
          this.program.programId,
          Buffer.from(getCompDefAccOffset("liquidation_check")).readUInt32LE()),
      })
      .signers([keeper])
      .rpc({ commitment: "confirmed" });

    console.log("liquidation_check queued:", sig);
    await awaitComputationFinalization(
      this.provider, computationOffset, this.program.programId, "confirmed");
    const event = await liqCheckEventPromise;
    return {
      encResult: new Uint8Array(event.encShouldLiquidate),
      nonce: new Uint8Array(event.nonce),
    };
  }

  decryptLiquidationResult(
    sharedSecret: Uint8Array,
    encResult: Uint8Array,
    nonce: Uint8Array
  ): boolean {
    const cipher = new RescueCipher(sharedSecret);
    const [result] = cipher.decrypt([encResult], nonce);
    return result === 1n;
  }

  async closePosition(
    trader: Keypair,
    feed: PriceFeed,
    sharedSecret: Uint8Array
  ): Promise<PnlResult> {
    const { cipher, publicKey } = await this._buildCipher();
    const nonce = randomBytes(16);
    const feedCiphertexts = cipher.encrypt(
      [feed.markPrice, feed.fundingRateBps, feed.timestamp], nonce);
    const computationOffset = new BN(randomBytes(8), "hex");
    const closeEventPromise = awaitEvent("positionClosedEvent");

    const sig = await this.program.methods
      .closePosition(
        computationOffset,
        Array.from(feedCiphertexts[0]),
        Array.from(feedCiphertexts[1]),
        Array.from(feedCiphertexts[2]),
        Array.from(publicKey),
        new BN(deserializeLE(nonce).toString())
      )
      .accountsPartial({
        trader: trader.publicKey,
        computationAccount: getComputationAccAddress(
          this.arciumEnv.arciumClusterOffset, computationOffset),
        clusterAccount: getClusterAccAddress(this.arciumEnv.arciumClusterOffset),
        mxeAccount: getMXEAccAddress(this.program.programId),
        mempoolAccount: getMempoolAccAddress(this.arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(this.arciumEnv.arciumClusterOffset),
        compDefAccount: getCompDefAccAddress(
          this.program.programId,
          Buffer.from(getCompDefAccOffset("close_position")).readUInt32LE()),
      })
      .signers([trader])
      .rpc({ commitment: "confirmed" });

    console.log("close_position queued:", sig);
    await awaitComputationFinalization(
      this.provider, computationOffset, this.program.programId, "confirmed");
    const event = await closeEventPromise;
    const pnlCipher = new RescueCipher(sharedSecret);
    const pnlNonce = new Uint8Array(event.pnlNonce);
    const [pnlValue, profitFlag] = pnlCipher.decrypt(
      [new Uint8Array(event.encPnl), new Uint8Array(event.encProfit)], pnlNonce);
    return {
      pnlUsdCents: pnlValue,
      isProfit: profitFlag === 1n,
    };
  }

  private async _buildCipher(): Promise<{
    cipher: RescueCipher;
    publicKey: Uint8Array;
    sharedSecret: Uint8Array;
  }> {
    const privateKey = x25519.x25519.utils.randomPrivateKey();
    const publicKey = x25519.x25519.getPublicKey(privateKey);
    const mxePublicKey = await getMXEPublicKeyWithRetry(
      this.provider, this.program.programId);
    const sharedSecret = x25519.x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);
    return { cipher, publicKey, sharedSecret };
  }
}
