# How Arcium Powers Private Perps

---

## 🧑‍💼 Simple Explanation (No Technical Background Needed)

Imagine you walk into a casino and place a bet — but the entire room can
see exactly how much you bet, which direction you chose, and how much
it would take to wipe you out. Other players could copy your bet, or
work together to push the odds against you until you lose everything.

That is what happens on every existing crypto trading platform today.
Your trade is public. Anyone can see it. Bots exploit it.

**Private Perps fixes this.**

When you open a trade on Private Perps, your position is locked inside
an encrypted vault powered by Arcium. Nobody — not other traders, not
bots, not even the protocol itself — can see your trade details.

The only thing that ever gets revealed is your final profit or loss,
and only you can read it.

Think of it like this:

Traditional perps:  Your trade is written on a public whiteboard.
Private Perps:      Your trade is locked in a vault. Only the result comes out.

---

## 🔒 What Stays Private vs What Is Revealed

| Your trade detail | What others see |
|-------------------|-----------------|
| How much you traded | 🔒 Nothing |
| Long or short | 🔒 Nothing |
| Your entry price | 🔒 Nothing |
| Your leverage | 🔒 Nothing |
| When you'd get liquidated | 🔒 Nothing |
| Your final profit/loss | Only **you** see this |

---

## ⚙️ How It Works (Plain English)

1. **You type your trade** — size, direction, leverage
2. **Your phone/computer scrambles it** into unreadable code before it goes anywhere
3. **Arcium's private computers** process your trade without ever unscrambling it — like a calculator that works on locked boxes
4. **Only your result (PnL)** comes back out, scrambled in a way only you can read
5. **You unscramble it** on your own device

Nobody in the middle — not Solana validators, not Arcium nodes, not liquidators — ever sees your trade.

---

## 🏗️ Technical Architecture (For Developers)

### The Problem in One Sentence

On every existing on-chain perps protocol, your position size, direction,
leverage, and liquidation price are fully visible to anyone — enabling
copy-trading, targeted liquidations, and MEV extraction.

### The Solution in One Sentence

Private Perps uses Arcium's MXE (Multi-party Computation eXecution Environment)
to run all sensitive trade logic over fully encrypted data, so nothing about
your position is ever visible on-chain — only your final PnL is revealed,
and only to you.

---

## How Arcium Is Used — Step by Step

### Step 1 — You encrypt your order locally

Before your order ever leaves your device, the TypeScript client performs
an x25519 ECDH key exchange with the Arcium MXE cluster to establish a
shared secret. Your order fields (size, price, direction, leverage) are
then encrypted using the Rescue cipher:

    privateKey   = x25519.randomPrivateKey()
    publicKey    = x25519.getPublicKey(privateKey)
    sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey)
    cipher       = new RescueCipher(sharedSecret)
    ciphertexts  = cipher.encrypt([size, price, isShort, leverage], nonce)

Nobody — not validators, not liquidators, not other traders — can read these values.

---

### Step 2 — Encrypted ciphertexts are submitted on-chain

The Solana program receives the encrypted ciphertexts and queues a
computation request to the Arcium network. At no point does plaintext
appear on-chain. Validators only see encrypted bytes.

---

### Step 3 — Arcium MXE runs the computation privately

Three circuits run entirely inside the encrypted execution environment:

**open_position**
Validates leverage bounds and creates an encrypted PositionInput.
The result is stored on-chain as a ciphertext — never as readable data.

**liquidation_check**
Takes the encrypted position and an encrypted price feed.
Returns only a single encrypted bit — 0 (safe) or 1 (liquidate).
The position details are never decrypted during this process.

**close_position**
Computes final PnL including leverage and funding costs.
This is the ONLY instruction that produces a revealed result —
and even then it is encrypted with the trader's shared secret
so only the trader can decrypt it.

---

### Step 4 — The callback delivers the encrypted result

When the MPC cluster finishes, it calls back into the Solana program
with the encrypted output. An observer on-chain sees only an opaque
ciphertext blob.

---

### Step 5 — Only the trader decrypts their PnL

The trader's client listens for the event and decrypts locally.
Nobody else can read this. Not the liquidator. Not the protocol. Not Arcium.

---

## What Is and Isn't Visible On-Chain

| Data | On-chain visibility | Who can read it |
|------|---------------------|-----------------|
| Position size | 🔒 Encrypted ciphertext | Nobody |
| Trade direction (long/short) | 🔒 Encrypted ciphertext | Nobody |
| Entry price | 🔒 Encrypted ciphertext | Nobody |
| Leverage | 🔒 Encrypted ciphertext | Nobody |
| Liquidation threshold | 🔒 Computed inside MXE | Nobody |
| Liquidation signal (0/1) | 🔒 Encrypted ciphertext | Only trader |
| Final PnL | 🔒 Encrypted ciphertext | Only trader |

---

## Why This Matters for Perps Specifically

**Copy-trading prevention**
Because position size and direction are encrypted, no bot or trader
can monitor your positions and mirror your trades.

**Liquidation hunting prevention**
Liquidation prices are computed inside the MXE. No actor can calculate
your liquidation threshold from on-chain data and push the price to trigger it.

**MEV elimination**
There is no position data to front-run. Oracle price feeds are also
encrypted before submission, preventing oracle manipulation attacks.

**Deeper liquidity**
When large traders can execute without revealing intent, they are
willing to place larger orders, improving market depth for everyone.

---

## Files Reference

| File | Purpose |
|------|---------|
| encrypted-ixs/src/lib.rs | Arcis MXE circuits — the 3 confidential instructions |
| programs/private_perps/src/lib.rs | Anchor Solana program — queues computations, handles callbacks |
| app/client.ts | TypeScript client — ECDH, encryption, submission, decryption |
| tests/private_perps.ts | Full lifecycle integration tests |
| app/ui/index.html | Trading frontend UI |
