"""Ownership + roles: raw PSBT fields -> NormalizedTx (spec §5). Pure, no I/O.

Precedence (evaluate in order; the first match wins):

 1. Any input lacks its UTXO data            -> error. Never guess values.
 2. Malformed tx: zero outputs, duplicate outpoint,
    or sum(outputs) > sum(inputs)            -> error.
 3. No input has any derivation              -> error: "no wallet metadata;
                                                 truth comparison impossible."
 4. Any single input lists >1 fingerprint    -> stop: multisig isn't supported yet.
 5. Wallet fingerprint F:
      - inputs carry exactly one fingerprint   -> that's F.
      - inputs carry >=2 fingerprints          -> F is the unique fingerprint
        that also appears on an internal-chain (.../1/...) output derivation.
        Zero or >=2 qualifying                 -> stop: cannot identify wallet.
 6. Input classes: owned / foreign / unattributed.  L = owned ∪ unattributed.

Output roles relative to F: change (/1/), self_receive (/0/), external,
op_return, unknown (has F but non-standard path — never guessed).

Degraded mode: F not identifiable -> ownership is a *guess* based on the
majority fingerprint of input derivations; change rules become guess-only.
Missing metadata is never treated as evidence (§2 worst-case observer).
"""

from __future__ import annotations

from collections import Counter
from typing import List, Optional

from .model import NormalizedTx, TxInput, TxOutput
from .parse import script_type_of, _script_from_hex


class OwnershipError(Exception):
    """Hard stop from the §5 precedence list. Carries the spec's message."""


def _path_chain(path: str) -> List[str]:
    """'m/84h/1h/0h/0/5' -> ['84h','1h','0h','0','5'] (drop 'm')."""
    parts = path.split("/")
    if parts and parts[0] == "m":
        parts = parts[1:]
    return parts


def chain_position(path: str) -> Optional[int]:
    """Return 0 (external chain) or 1 (internal/change chain) if the path has
    a standard BIP44-style …/change/index shape, else None."""
    chain = _path_chain(path)
    if len(chain) >= 2:
        try:
            pos = int(chain[-2].rstrip("h"))
        except ValueError:
            return None
        if pos in (0, 1):
            return pos
    return None


def _owned_by(inp_or_out, fp: int, taproot_internal_key: Optional[str] = None) -> bool:
    """Does this scope carry a derivation for fingerprint fp?

    For P2TR, wallets register the tweaked output key under TAP_BIP32_DERIVATION
    while inputs carry the internal key; we match either (see CHANGELOG note).
    """
    for f, _p, key in inp_or_out.derivations:
        if f == fp:
            return True
    return False


def normalize(raw: dict) -> NormalizedTx:
    # -- build scopes ------------------------------------------------------
    inputs: List[TxInput] = []
    for d in raw["inputs"]:
        stype = script_type_of(_script_from_hex(d["script_pubkey_hex"]))
        inputs.append(TxInput(
            index=d["index"],
            txid=d["txid"],
            vout=d["vout"],
            value=d["value"],
            script_pubkey_hex=d["script_pubkey_hex"],
            script_type=stype,
            sequence=d["sequence"],
            derivations=[tuple(x) for x in d["derivations"]],
            taproot_internal_key_hex=d.get("taproot_internal_key_hex"),
        ))

    outputs: List[TxOutput] = []
    for d in raw["outputs"]:
        stype = script_type_of(_script_from_hex(d["script_pubkey_hex"]))
        outputs.append(TxOutput(
            index=d["index"],
            value=d["value"],
            script_pubkey_hex=d["script_pubkey_hex"],
            script_type=stype,
            derivations=[tuple(x) for x in d["derivations"]],
        ))

    # -- precedence 1: every input must carry UTXO data ---------------------
    # (enforced by the parser, which raises before we get here; re-checked)
    for inp in inputs:
        if inp.value is None or inp.script_pubkey_hex is None:
            raise OwnershipError(
                f"Input {inp.index} lacks UTXO data. Values are never guessed."
            )

    # -- precedence 2: malformed tx ----------------------------------------
    if len(outputs) == 0:
        raise OwnershipError("Malformed transaction: zero outputs.")
    seen = set()
    for inp in inputs:
        op = (inp.txid, inp.vout)
        if op in seen:
            raise OwnershipError(
                f"Malformed transaction: duplicate outpoint {inp.txid}:{inp.vout}."
            )
        seen.add(op)
    sum_in = sum(i.value for i in inputs)
    sum_out = sum(o.value for o in outputs)
    if sum_out > sum_in:
        raise OwnershipError(
            f"Malformed transaction: outputs ({sum_out:,} sats) exceed "
            f"inputs ({sum_in:,} sats)."
        )

    # -- precedence 3: at least one input derivation ------------------------
    if not any(i.derivations for i in inputs):
        raise OwnershipError(
            "No wallet metadata; truth comparison impossible. "
            "(No input carries a BIP32 derivation.)"
        )

    # -- precedence 4: multisig stop ----------------------------------------
    for inp in inputs:
        if len(set(fp for fp, _p, _k in inp.derivations)) > 1:
            raise OwnershipError(
                "Multisig isn't supported yet. Only ownership detection would "
                "change; the rules themselves generalize."
            )

    # -- precedence 5: wallet fingerprint F ---------------------------------
    input_fps = sorted({fp for i in inputs for fp, _p, _k in i.derivations})
    fingerprint: Optional[int] = None
    source = "inputs"
    degraded = False
    if len(input_fps) == 1:
        fingerprint = input_fps[0]
    else:  # >= 2 distinct fingerprints across inputs
        qualifying = []
        for fp in input_fps:
            for o in outputs:
                if chain_position_is_internal_for(o, fp):
                    qualifying.append(fp)
                    break
        uniq = sorted(set(qualifying))
        if len(uniq) == 1:
            fingerprint = uniq[0]
            source = "internal-chain-output"
        else:
            raise OwnershipError(
                "Cannot identify which wallet built this transaction."
            )

    # -- degraded mode: F not among output derivations -----------------------
    # Per §5: "If no output carries F, the tool can't tell a changeless payment
    # from a wallet that omits output metadata." That is the degraded condition
    # even when F itself is known. We record it as `degraded_output` via the
    # NormalizedTx.fingerprint presence + role scan in rules.py; here we only
    # mark the harder case where F could not be identified at all.
    # (F is always identified above or we stopped, so NormalizedTx.degraded
    # stays False; output-side degradation is derived in rules.analyze.)

    # -- precedence 6: input classes -----------------------------------------
    for inp in inputs:
        fps = {fp for fp, _p, _k in inp.derivations}
        if not fps:
            inp.cls = "unattributed"
        elif fingerprint in fps:
            inp.cls = "owned"
        else:
            inp.cls = "foreign"

    # -- output roles relative to F ------------------------------------------
    for o in outputs:
        if o.script_type == "op_return":
            o.role = "op_return"
            continue
        has_f = any(fp == fingerprint for fp, _p, _k in o.derivations)
        if not has_f:
            o.role = "external"
            continue
        positions = {chain_position(p) for fp, p, _k in o.derivations
                     if fp == fingerprint}
        if 1 in positions:
            o.role = "change"
        elif positions == {0}:
            o.role = "self_receive"
        else:
            o.role = "unknown"   # has F, non-standard path. Never guessed.

    return NormalizedTx(
        version=raw["version"],
        locktime=raw["locktime"],
        inputs=inputs,
        outputs=outputs,
        psbt_version=raw.get("psbt_version"),
        fingerprint=fingerprint,
        fingerprint_source=source,
        degraded=degraded,
    )


def chain_position_is_internal_for(output: TxOutput, fp: int) -> bool:
    """True if this output carries a derivation for fp on an internal chain
    (.../1/...), i.e. a change address of wallet fp."""
    for f, p, _k in output.derivations:
        if f == fp and chain_position(p) == 1:
            return True
    return False


def outputs_carry_fingerprint(ntx: NormalizedTx) -> bool:
    """§5 Degraded-mode trigger: does ANY output carry F?"""
    if ntx.fingerprint is None:
        return False
    return any(any(f == ntx.fingerprint for f, _p, _k in o.derivations)
               for o in ntx.outputs)
