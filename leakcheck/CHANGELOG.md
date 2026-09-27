# CHANGELOG — LeakCheck v4

All spec changes from the frozen v4 document (FROZEN Sun Sep 27, 2026) must be
logged here with their reason. No wording or feature changes after the freeze
except those triggered by a failed Day-0 check or a test-exposed contradiction.

## Entries

### 2026-09-27 — Day-0 environment substitution (§11)
**Reason (Day-0 check):** This build environment has no internet access to a
signet faucet and no Sparrow installation, so the §11 checks that require them
(faucet coins, "Sparrow can export base64", recording "the PSBT version
Sparrow emits") cannot be executed here. They remain open items for the human
builder; NOTES.md records exactly which passed offline and which are blocked.
**Substitution:** Day-0's parseability requirement is verified against a
synthetic signet-mirroring fixture generator (`tests/fixtures/gen_psbt.py`)
that builds real PSBTs with embit, including a Taproot input, derivations,
UTXO data, PayJoin-style foreign inputs, signed PSBTs, and a PSBTv2. The
generator doubles as the fixture factory for §10 tests.

### 2026-09-27 — §5 "Taproot: the TAP_BIP32_DERIVATION fields" clarified in implementation
**Reason (test-revealed ambiguity, not a spec change):** For key-path-only
P2TR outputs the wallet registers the *tweaked* output key under
PSBT_OUT_TAP_BIP32_DERIVATION, which does not match the internal key stored
in the input's PSBT_IN_TAP_INTERNAL_KEY. Ownership therefore matches a P2TR
derivation if its pubkey equals the script key OR the input's taproot internal
key. This only makes detection strictly better; rule semantics are unchanged.

No other deviations from the frozen spec.
