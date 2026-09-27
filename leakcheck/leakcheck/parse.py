"""PSBT parsing glue: bytes/base64 -> raw fields (spec §4, §5).

Uses the maintained `embit` library for all serialization — no hand-rolled
parsing. PSBT_GLOBAL_XPUB is read by the library but deliberately dropped
here and never surfaced to callers (§3: ignored and never displayed).
"""

from __future__ import annotations

import base64
import binascii
from typing import List, Optional

from embit.psbt import PSBT
from embit.base import EmbitError


class ParseError(Exception):
    """Raised for malformed PSBTs or missing required data (§5 precedence 1)."""


def _looks_like_base64(s: str) -> bool:
    t = "".join(s.split())
    if len(t) < 32 or len(t) % 4 != 0:
        return False
    try:
        base64.b64decode(t, validate=True)
        return True
    except (binascii.Error, ValueError):
        return False


def load_psbt(data) -> PSBT:
    """Accept binary PSBT bytes, or a str containing base64 or hex.

    Returns an embit PSBT object. Raises ParseError on anything unparseable.
    """
    if isinstance(data, str):
        text = data.strip()
        candidate = text
        raw = None
        # try base64 first (Sparrow copy/paste format)
        if _looks_like_base64(candidate):
            try:
                raw = base64.b64decode("".join(candidate.split()), validate=True)
            except (binascii.Error, ValueError):
                raw = None
        if raw is None:
            # try hex
            try:
                raw = bytes.fromhex(candidate)
            except ValueError:
                raise ParseError(
                    "Could not decode input: expected a base64 or hex PSBT."
                )
        data = raw

    data = bytes(data)
    if not data.startswith(b"psbt\xff"):
        raise ParseError("Not a PSBT: missing magic bytes 'psbt\\xff'.")
    try:
        psbt = PSBT.parse(data)
    except (EmbitError, Exception) as e:  # embit raises assorted subclasses
        raise ParseError(f"Malformed PSBT: {e}") from e
    return psbt


def normalize_derivation_path(path: List[int]) -> str:
    """BIP32 index list -> 'm/44h/1h/0h/0/3' string ('h' for hardened)."""
    parts = ["m"]
    for i in path:
        if i >= 0x80000000:
            parts.append(f"{i - 0x80000000}h")
        else:
            parts.append(str(i))
    return "/".join(parts)


def script_type_of(script) -> str:
    """Map an embit Script to the §5 type vocabulary.

    p2pkh, p2sh, p2wpkh, p2wsh, p2tr, op_return, unknown.
    P2SH counts as one type; anything unrecognized is 'unknown'
    (and 'unknown' is never evidence).
    """
    if script is None:
        return "unknown"
    data = bytes(script.data) if hasattr(script, "data") else bytes(script)
    # OP_RETURN anywhere at the start with standard shape: 6a [push...]
    if len(data) >= 1 and data[0] == 0x6A:
        return "op_return"
    st = None
    try:
        st = script.script_type()
    except Exception:
        st = None
    if st in ("p2pkh", "p2sh", "p2wpkh", "p2wsh", "p2tr"):
        return st
    return "unknown"


# ---------------------------------------------------------------------------
# Raw extraction into plain dicts (parser boundary; normalize.py owns meaning)
# ---------------------------------------------------------------------------

def extract_raw(psbt: PSBT) -> dict:
    """Pull every field the analyzer needs into plain structures.

    Deliberately does NOT touch psbt.xpubs (PSBT_GLOBAL_XPUB) beyond dropping
    it, so downstream code cannot display it.
    """
    tx = psbt.tx
    inputs: List[dict] = []
    for i, inp in enumerate(psbt.inputs):
        utxo = None
        if inp.witness_utxo is not None:
            utxo = inp.witness_utxo
        elif inp.non_witness_utxo is not None:
            try:
                utxo = inp.non_witness_utxo.vout[inp.vout]
            except (IndexError, TypeError):
                utxo = None
        if utxo is None:
            raise ParseError(
                f"Input {i} lacks UTXO data (no witness_utxo / non_witness_utxo). "
                "Values are never guessed."
            )
        derivations = []
        for pub, d in inp.bip32_derivations.items():
            derivations.append((
                int.from_bytes(bytes(d.fingerprint), "big"),
                normalize_derivation_path(d.derivation),
                pub.serialize().hex(),
            ))
        for pub, (leafs, d) in inp.taproot_bip32_derivations.items():
            derivations.append((
                int.from_bytes(bytes(d.fingerprint), "big"),
                normalize_derivation_path(d.derivation),
                pub.serialize().hex(),
            ))
        internal_key = None
        if inp.taproot_internal_key is not None:
            internal_key = inp.taproot_internal_key.serialize().hex()
        inputs.append({
            "index": i,
            "txid": inp.txid.hex() if hasattr(inp.txid, "hex") else bytes(inp.txid).hex(),
            "vout": inp.vout,
            "value": int(utxo.value),
            "script_pubkey_hex": bytes(utxo.script_pubkey).hex(),
            "sequence": int(inp.sequence),
            "derivations": derivations,
            "taproot_internal_key_hex": internal_key,
        })

    outputs: List[dict] = []
    for j, out in enumerate(psbt.outputs):
        derivations = []
        for pub, d in out.bip32_derivations.items():
            derivations.append((
                int.from_bytes(bytes(d.fingerprint), "big"),
                normalize_derivation_path(d.derivation),
                pub.serialize().hex(),
            ))
        for pub, (leafs, d) in out.taproot_bip32_derivations.items():
            derivations.append((
                int.from_bytes(bytes(d.fingerprint), "big"),
                normalize_derivation_path(d.derivation),
                pub.serialize().hex(),
            ))
        outputs.append({
            "index": j,
            "value": int(out.value),
            "script_pubkey_hex": bytes(out.script_pubkey).hex(),
            "derivations": derivations,
        })

    return {
        "version": int(tx.version),
        "locktime": int(tx.locktime),
        "psbt_version": psbt.version,   # None => v0
        "inputs": inputs,
        "outputs": outputs,
    }


def parse_input(data) -> dict:
    """bytes | base64 str -> raw field dict. Single entry point for CLI/UI."""
    psbt = load_psbt(data)
    raw = extract_raw(psbt)
    raw["script_types_in"] = [
        script_type_of(_script_from_hex(d["script_pubkey_hex"]))
        for d in raw["inputs"]
    ]
    raw["script_types_out"] = [
        script_type_of(_script_from_hex(d["script_pubkey_hex"]))
        for d in raw["outputs"]
    ]
    return raw


def _script_from_hex(h: str):
    from embit.script import Script
    return Script(bytes.fromhex(h))
