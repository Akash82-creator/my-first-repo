# NOTES — Day 0 (Sun Sep 27, 2026)

## §11 Day-0 checks — results

1. **Deadline.** BOSS Battle (Bitshala) Overview page says the hackathon ends
   **Mon Oct 5, 2026**. The exact cutoff time and timezone could NOT be
   verified from this environment (no browser/explorer access allowed for the
   tool anyway, and this sandbox has no access to the listing beyond the spec).
   **STATUS: OPEN — builder must confirm on the live Overview page tonight and
   record date + time + timezone + submission requirements here.** Schedule
   assumes submission Sun Oct 4 with Oct 5 as buffer.

2. **Library parses a signet-mirroring PSBT offline, incl. Taproot input.**
   **PASS.** `embit` (installed via pip, pure-python fallback secp256k1 — no
   compiled deps) parses v0 and v2 PSBTs from binary and base64, exposes
   witness_utxo / non_witness_utxo, bip32_derivations and
   taproot_bip32_derivations. Verified by `tests/test_parser.py`.
   Recorded library: **embit**. Recorded PSBT versions exercised: **v0 and v2**
   (Sparrow's actual emitted version remains an open item — see #1/#2 env note
   in CHANGELOG; parser handles both).

3. **Fixture has input UTXO data and output derivations for the change output.**
   **PASS** (for the synthetic fixtures). Real-Sparrow confirmation is blocked
   in this environment (see CHANGELOG Day-0 entry). If a future real Sparrow
   PSBT lacks output derivations, degraded mode (§5) becomes the default path —
   it is implemented and tested, so the pitch survives either way.

4. **Signet faucet / self-sends to create small UTXOs.** **BLOCKED** in this
   environment (no network to a faucet, no Sparrow). Builder action: request
   signet coins tonight (Day-0 deliverable), then run
   `tests/fixtures/gen_psbt.py --write-samples` locally when real coins exist.
   Demo PSBTs A/B are generated from synthetic signet-height data meanwhile and
   are labeled as such.

5. **Sparrow exports a PSBT as copyable base64.** **UNVERIFIED here**; the tool
   accepts base64 paste regardless (tested both encodings).

6. **Prior art search.** Cannot be performed from this environment. **OPEN.**
   Per §11, no prior-art claims appear in README/RULES until done. Candidate
   queries for the builder: "pre-broadcast PSBT privacy checker", "coinjoin
   heuristic analyzer local", "wallet heuristics before broadcast". (Related
   public post-hoc analyzers exist academically — e.g. peeling-chain research —
   but the pre-broadcast truth-comparison framing is what §14 defends as new.)

7. **BIP78 receiver derivation data on added inputs.** **OPEN** (needs network +
   a receiver). Non-blocking: precedence step 5 handles both cases — a foreign
   input without derivation is `unattributed`, with derivation it is
   `foreign`; tested both ways (A1/A2 fixtures).

## Environment facts

- Python 3.12.10 (spec targets 3.11; code is 3.11-compatible).
- Installed: embit, fastapi, uvicorn, pytest, httpx (test client).
- Network: package installs worked at setup time; runtime analysis performs
  zero network requests (enforced by `tests/test_privacy.py`).
