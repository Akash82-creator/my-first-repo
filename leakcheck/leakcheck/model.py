"""Data model for the normalized transaction and rule findings.

Pure data; no I/O anywhere in this module (spec §4).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import List, Optional

# ---------------------------------------------------------------------------
# Findings
# ---------------------------------------------------------------------------

KINDS = ("warning", "favorable", "neutral")


@dataclass
class Finding:
    """Finding schema, spec §7."""

    rule: str
    kind: str                      # warning | favorable | neutral
    observation: str               # deterministic fact, with values
    inference: str                 # what an observer would conclude
    action: str                    # what you can still change before broadcasting
    limits: str                    # what this check cannot tell you
    impact: Optional[str] = None   # high | medium | low | None
    confidence: Optional[str] = None  # likely | possible | None
    applicable: bool = True        # False => counted under "not applicable"

    def as_dict(self) -> dict:
        return {
            "rule": self.rule,
            "kind": self.kind,
            "impact": self.impact,
            "confidence": self.confidence,
            "observation": self.observation,
            "inference": self.inference,
            "action": self.action,
            "limits": self.limits,
            "applicable": self.applicable,
        }


@dataclass
class ChangeGuess:
    """A change-heuristic guess: one spendable output, or none (spec §6)."""

    rule: str
    guess: Optional[int]           # output index, or None for "no guess"
    strong: bool = False           # only round-payment ever sets this True
    note: str = ""


@dataclass
class AnalysisResult:
    findings: List[Finding] = field(default_factory=list)
    degraded: bool = False         # wallet fingerprint could not be identified

    def by_rule(self, rule: str) -> Optional[Finding]:
        for f in self.findings:
            if f.rule == rule:
                return f
        return None

    @property
    def warnings(self) -> List[Finding]:
        return [f for f in self.findings if f.kind == "warning"]

    @property
    def favorables(self) -> List[Finding]:
        return [f for f in self.findings if f.kind == "favorable"]

    @property
    def neutrals(self) -> List[Finding]:
        return [f for f in self.findings if f.kind == "neutral"]


# ---------------------------------------------------------------------------
# Normalized transaction
# ---------------------------------------------------------------------------

SCRIPT_TYPES = ("p2pkh", "p2sh", "p2wpkh", "p2wsh", "p2tr", "op_return", "unknown")


def fmt_sats(v: int) -> str:
    return f"{v:,} sats"


@dataclass
class TxInput:
    index: int
    txid: str                      # display order (big-endian hex)
    vout: int
    value: int                     # sats, from UTXO data (never guessed)
    script_pubkey_hex: str
    script_type: str               # one of SCRIPT_TYPES
    sequence: int
    derivations: List[tuple] = field(default_factory=list)
    # derivations: list of (fingerprint_int, path_str, pubkey_hex); path_str is
    # e.g. "m/84h/1h/0h/0/5" (normalized with leading "m").
    taproot_internal_key_hex: Optional[str] = None
    cls: str = "unattributed"      # owned | foreign | unattributed (set by normalize)

    @property
    def fingerprints(self):
        return sorted({fp for fp, _p, _k in self.derivations})

    @property
    def max_path_len(self):
        return max((len(p.split("/")) - 1 for _fp, p, _k in self.derivations), default=0)

    def paths_for(self, fp: int):
        return [p for f, p, _k in self.derivations if f == fp]

    def keys_for(self, fp: int):
        return [k for f, _p, k in self.derivations if f == fp]


@dataclass
class TxOutput:
    index: int
    value: int                     # sats
    script_pubkey_hex: str
    script_type: str
    derivations: List[tuple] = field(default_factory=list)
    role: str = "external"         # change | self_receive | external | op_return | unknown

    @property
    def fingerprints(self):
        return sorted({fp for fp, _p, _k in self.derivations})

    def paths_for(self, fp: int):
        return [p for f, p, _k in self.derivations if f == fp]


@dataclass
class NormalizedTx:
    version: int                   # nVersion of the unsigned tx
    locktime: int
    inputs: List[TxInput]
    outputs: List[TxOutput]
    psbt_version: Optional[int] = None   # None => v0
    fingerprint: Optional[int] = None    # wallet fingerprint F
    fingerprint_source: str = "inputs"   # inputs | internal-chain-output
    degraded: bool = False               # F not identifiable (§5 Degraded mode)

    # -- helpers -----------------------------------------------------------

    def input(self, i: int) -> TxInput:
        return self.inputs[i]

    def output(self, i: int) -> TxOutput:
        return self.outputs[i]

    @property
    def linkable(self) -> List[TxInput]:
        """L = owned ∪ unattributed inputs; foreign inputs excluded (§5)."""
        return [inp for inp in self.inputs if inp.cls != "foreign"]

    @property
    def owned_inputs(self) -> List[TxInput]:
        return [inp for inp in self.inputs if inp.cls == "owned"]

    @property
    def foreign_inputs(self) -> List[TxInput]:
        return [inp for inp in self.inputs if inp.cls == "foreign"]

    @property
    def unattributed_inputs(self) -> List[TxInput]:
        return [inp for inp in self.inputs if inp.cls == "unattributed"]

    @property
    def spendable_outputs(self) -> List[TxOutput]:
        """All outputs except op_return and zero-value outputs (§5)."""
        return [o for o in self.outputs
                if o.script_type != "op_return" and o.value > 0]

    @property
    def true_change(self) -> Optional[TxOutput]:
        """The single change output if exactly one exists and F is known."""
        if self.fingerprint is None:
            return None
        ch = [o for o in self.outputs if o.role == "change"]
        return ch[0] if len(ch) == 1 else None

    def describe_input(self, inp: TxInput) -> str:
        short = inp.txid[:16] + ":" + str(inp.vout)
        return f"input {inp.index} ({short}, {fmt_sats(inp.value)}, {inp.script_type})"

    def describe_output(self, out: TxOutput) -> str:
        return f"output {out.index} ({fmt_sats(out.value)}, {out.script_type}, role={out.role})"
