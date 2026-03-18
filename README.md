# 🔒 Private Perps — Confidential Perpetuals on Solana × Arcium

> Perpetual futures where **positions, order sizes, leverage, and liquidation checks are fully private**. Only final PnL is revealed — and only to the trader.

Built for the Arcium Developer Portal — Private Perps Bounty.

---

## The Problem

Traditional on-chain perps expose everything:

| What is visible on-chain | Impact |
|--------------------------|--------|
| Position size & direction | Copy-trading, front-running |
| Leverage used | Targeted liquidation hunting |
| Entry price | MEV bots extract value |
| Liquidation threshold | Adversarial price pushing |

---

## The Solution: Arcium MXE

Private Perps uses Arcium's MXE to run all sensitive computations over fully encrypted data. Only final PnL is revealed — and only to the trader.

---

## Architecture

- encrypted-ixs/src/lib.rs — Arcis MXE circuits (runs inside MPC)
- programs/private_perps/src/lib.rs — Anchor Solana program
- app/client.ts — TypeScript client
- tests/private_perps.ts — Integration tests

---

## Privacy Guarantees

| Action | On-chain | Trader sees |
|--------|----------|-------------|
| Open position | Encrypted ciphertext | Size, price, direction, leverage |
| Liquidation check | Encrypted boolean | Whether at risk |
| Close position | Encrypted PnL blob | Exact PnL in USD |

---

## Quick Start

arcium build
arcium test
arcium deploy --cluster devnet

---

## License

MIT
