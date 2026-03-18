import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair } from "@solana/web3.js";
import { expect } from "chai";
import * as os from "os";
import { readFileSync } from "fs";
import { PrivatePerps } from "../target/types/private_perps";
import { PrivatePerpsClient } from "../app/client";

function readKpJson(path: string): Keypair {
  const raw = JSON.parse(readFileSync(path, "utf-8"));
  return Keypair.fromSecretKey(new Uint8Array(raw));
}

describe("Private Perps — Full Lifecycle", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program  = anchor.workspace.PrivatePerps as Program<PrivatePerps>;
  const provider = anchor.getProvider() as anchor.AnchorProvider;
  const client   = new PrivatePerpsClient(program, provider);

  const trader = readKpJson(`${os.homedir()}/.config/solana/id.json`);
  const keeper = Keypair.generate();

  let traderSharedSecret: Uint8Array;

  before("Airdrop keeper", async () => {
    const sig = await provider.connection.requestAirdrop(
      keeper.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(sig);
  });

  it("Initialises computation definitions", async () => {
    await client.initCompDefs(trader);
    console.log("Comp defs initialised");
  });

  it("Opens a long position — nothing revealed on-chain", async () => {
    const { sharedSecret } = await client.openPosition(trader, {
      size:       1_000_000n,
      limitPrice: 15_000_00n,
      isShort:    false,
      leverage:   10,
    });
    traderSharedSecret = sharedSecret;
    console.log("Position opened — size, direction, leverage all encrypted");
  });

  it("Liquidation check — price at $145 — should NOT liquidate", async () => {
    const feed = {
      markPrice:      14_500_00n,
      fundingRateBps: 10n,
      timestamp:      BigInt(Math.floor(Date.now() / 1000)),
    };
    const { encResult, nonce } = await client.liquidationCheck(
      keeper, trader.publicKey, feed);
    const shouldLiquidate = client.decryptLiquidationResult(
      traderSharedSecret, encResult, nonce);
    expect(shouldLiquidate).to.be.false;
    console.log("Safe — position still open");
  });

  it("Liquidation check — price at $135 — should liquidate", async () => {
    const feed = {
      markPrice:      13_500_00n,
      fundingRateBps: 10n,
      timestamp:      BigInt(Math.floor(Date.now() / 1000)),
    };
    const { encResult, nonce } = await client.liquidationCheck(
      keeper, trader.publicKey, feed);
    const shouldLiquidate = client.decryptLiquidationResult(
      traderSharedSecret, encResult, nonce);
    expect(shouldLiquidate).to.be.true;
    console.log("Should liquidate");
  });

  it("Closes position at $165 — PnL revealed only to trader", async () => {
    const feed = {
      markPrice:      16_500_00n,
      fundingRateBps: 10n,
      timestamp:      BigInt(Math.floor(Date.now() / 1000)),
    };
    const pnl = await client.closePosition(trader, feed, traderSharedSecret);
    console.log(`PnL: ${pnl.isProfit ? "+" : "-"}${Number(pnl.pnlUsdCents) / 100} USD`);
    expect(pnl.isProfit).to.be.true;
    expect(pnl.pnlUsdCents).to.be.greaterThan(0n);
    console.log("Position closed — PnL decrypted by trader only");
  });
});
