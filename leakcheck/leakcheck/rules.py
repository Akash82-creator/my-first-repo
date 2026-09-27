"""analyze(NormalizedTx) -> list[Finding]  (spec §6). Pure function, no I/O.

Hand-written by the builder per §12. Do not change rule directions, the
precedence order, or the verdict algorithm (§10 tests are the spec).

Two degradation axes, kept distinct on purpose:
  * ntx.degraded        — ownership itself unverifiable (F not identifiable).
                          Input rules stay truth-checked; change rules run
                          guess-only ("observer guess, unverifiable");
                          changeless and self-transfer are disabled.
  * outputs_unverifiable — F known but no output carries F (§5 Degraded mode):
                          the tool can't tell a changeless payment from a
                          wallet that omits output metadata. It says so;
                          changeless and self-transfer are disabled; change
                          heuristics report "not applicable" (no true change
                          to compare against).
"""

from __future__ import annotations

from collections import Counter
from typing import List, Optional

from .config import (
    ROUND_LADDER,
    STRONG_ROUND,
    SMALL_UTXO,
    LINKAGE_HIGH_THRESHOLD,
)
from .model import ChangeGuess, Finding, NormalizedTx, fmt_sats
from .normalize import outputs_carry_fingerprint

# ---------------------------------------------------------------------------
# The full catalog of checks (§10 "honest check counts").
#   applied     — fired or evaluated with a result
#   na          — evaluated, not applicable to this transaction
#   unavailable — cannot run without wallet history / network data (§9)
# ---------------------------------------------------------------------------

RULE_CATALOG = [
    # input & reuse rules
    "common-input-linkage", "foreign-input", "input-address-reuse",
    "sender-reuse", "recipient-reuse", "consolidation", "self-transfer",
    "small-input", "changeless",
    # change heuristics
    "round-payment", "script-type-match", "optimal-change",
    # fingerprint panel (§8) is display-only: no rule logic, never counted.
]

OPTIMAL_CHANGE_ENABLED = True  # optional rule; ships only while Day-4 stays green


def _na(rule: str, why: str) -> Finding:
    return Finding(
        rule=rule, kind="neutral", impact=None, confidence=None,
        observation=f"Not applicable: {why}",
        inference="No signal.", action="Nothing to change.",
        limits="This check did not apply to this transaction.",
        applicable=False,
    )


# ---------------------------------------------------------------------------
# Rounding ladder
# ---------------------------------------------------------------------------

def roundness(v: int) -> int:
    """First unit in ROUND_LADDER that divides v, else 0.

    Roundness is defined for positive amounts only; zero-value outputs are
    excluded from spendable sets upstream (§5), so they never reach here.
    """
    if v <= 0:
        return 0
    for unit in ROUND_LADDER:
        if v % unit == 0:
            return unit
    return 0


# ---------------------------------------------------------------------------
# Input and reuse rules
# ---------------------------------------------------------------------------

def rule_common_input_linkage(ntx: NormalizedTx) -> Finding:
    L = ntx.linkable
    scripts = {i.script_pubkey_hex for i in L}
    if len(L) >= 2 and len(scripts) >= 2:
        impact = "high" if len(scripts) >= LINKAGE_HIGH_THRESHOLD else "medium"
        all_owned = not ntx.unattributed_inputs
        if all_owned:
            confidence = "likely"
            caveat = ""
        else:
            confidence = "possible"
            caveat = (" Some inputs carry no derivation and may belong to "
                      "another party; this tool cannot tell.")
        return Finding(
            rule="common-input-linkage", kind="warning",
            impact=impact, confidence=confidence,
            observation=(f"{len(L)} linkable inputs use {len(scripts)} distinct "
                         f"scriptPubKeys.{caveat}"),
            inference=("An observer would conclude these coins were under one "
                       "controller at signing time (common-input-ownership heuristic)."),
            action=("Rebuild with coin control before broadcasting: pay from "
                    "fewer coins, or split the payment into separate "
                    "transactions."),
            limits=("Ownership heuristics can be wrong (CoinJoin, PayJoin); "
                    "this tool compares them against your wallet's own "
                    "metadata, not the chain."),
        )
    return _na("common-input-linkage",
               "fewer than two linkable inputs, or all share one scriptPubKey")


def rule_foreign_input(ntx: NormalizedTx) -> Finding:
    foreign = ntx.foreign_inputs
    if foreign:
        fps = ", ".join("#%x" % fp for fp in
                        sorted({f for i in foreign for f, _, _ in i.derivations}))
        return Finding(
            rule="foreign-input", kind="favorable",
            impact=None, confidence="likely",
            observation=(f"{len(foreign)} input(s) carry a fingerprint other "
                         f"than yours ({fps})."),
            inference=("An observer would link "
                       f"{len(foreign)} input(s) belonging to another party to "
                       "your coins. That part of the inference is false. Links "
                       "among your own inputs are unaffected (see the warning "
                       "above, if any)."),
            action=("Nothing to do: this is the effect PayJoin-style mixed "
                    "inputs have. Keep it."),
            limits=("The other party's presence is visible; their coins become "
                    "linked to yours in the observer's graph even though the "
                    "attribution is wrong."),
        )
    return _na("foreign-input", "no input carries a different fingerprint")


def rule_input_address_reuse(ntx: NormalizedTx) -> Finding:
    L = ntx.linkable
    counts = Counter(i.script_pubkey_hex for i in L)
    reused = {s: c for s, c in counts.items() if c >= 2}
    if reused:
        n = sum(reused.values())
        return Finding(
            rule="input-address-reuse", kind="warning",
            impact="medium", confidence="likely",
            observation=(f"{n} inputs in the linkable set spend from the same "
                         f"address(es) more than once ({len(reused)} address(es) affected)."),
            inference=("An observer learns this address was funded more than "
                       "once, and links every spend of it."),
            action=("Already on-chain: this transaction cannot fix it. Don't "
                    "combine these coins with others; use fresh addresses "
                    "going forward."),
            limits=("Reuse across your whole wallet history is invisible to a "
                    "PSBT check — see the 'not checked' list."),
        )
    return _na("input-address-reuse", "no address is spent twice in this tx")


def rule_sender_reuse(ntx: NormalizedTx) -> Finding:
    L = ntx.linkable
    prevout_scripts = {i.script_pubkey_hex for i in L}
    owned_out_scripts = Counter(
        o.script_pubkey_hex for o in ntx.outputs
        if any(f == ntx.fingerprint for f, _, _ in o.derivations)
    )
    hits_prevout = [o.index for o in ntx.outputs
                    if o.script_pubkey_hex in prevout_scripts
                    and o.script_type != "op_return"]
    dup_owned = [s for s, c in owned_out_scripts.items() if c >= 2]
    if hits_prevout or dup_owned:
        parts = []
        if hits_prevout:
            parts.append(f"output(s) {hits_prevout} reuse an input's "
                         "scriptPubKey as a sender address")
        if dup_owned:
            parts.append(f"{len(dup_owned)} owned scriptPubKey(s) appear on "
                         "more than one output")
        return Finding(
            rule="sender-reuse", kind="warning",
            impact="high", confidence="likely",
            observation="; ".join(parts) + ".",
            inference=("Sender-address reuse lets an observer tie the new "
                       "outputs back to the spending address and forward "
                       "through it — the classic change-identification lever."),
            action=("Use a fresh change address before broadcasting (rotate "
                    "the internal chain index in your wallet)."),
            limits=("If the reused script belongs to a PayJoin counterpart, "
                    "the leak is partly theirs; this tool cannot attribute "
                    "intent."),
        )
    return _na("sender-reuse", "no output reuses a sender address")


def rule_recipient_reuse(ntx: NormalizedTx) -> Finding:
    external = [o for o in ntx.outputs if o.role == "external"
                and o.script_type != "op_return" and o.value > 0]
    counts = Counter(o.script_pubkey_hex for o in external)
    dup = {s: c for s, c in counts.items() if c >= 2}
    foreign_scripts = {i.script_pubkey_hex for i in ntx.foreign_inputs}
    reused_foreign = [o.index for o in external
                      if o.script_pubkey_hex in foreign_scripts]
    if dup or reused_foreign:
        parts = []
        if dup:
            parts.append(f"{len(dup)} recipient scriptPubKey(s) receive more "
                         "than once in this tx")
        if reused_foreign:
            parts.append(f"output(s) {reused_foreign} pay to the same address "
                         "a foreign input came from")
        return Finding(
            rule="recipient-reuse", kind="warning",
            impact="medium", confidence="likely",
            observation="; ".join(parts) + ".",
            inference=("An observer merges everything sent to a repeated "
                       "recipient address into one entity. It mainly harms "
                       "the recipient's privacy."),
            action=("It is the recipient's choice of address; you can ask "
                    "them for a fresh one, but don't alter the invoice amount."),
            limits=("Your own change location is NOT revealed by this finding; "
                    "reuse among the recipient's addresses is their exposure."),
        )
    return _na("recipient-reuse", "each recipient address appears once")


def rule_consolidation(ntx: NormalizedTx) -> Finding:
    L = ntx.linkable
    spend = ntx.spendable_outputs
    single_owned = (len(spend) == 1
                    and any(f == ntx.fingerprint for f, _, _ in spend[0].derivations))
    if len(L) >= 2 and not ntx.foreign_inputs and single_owned:
        return Finding(
            rule="consolidation", kind="warning",
            impact="high", confidence="likely",
            observation=(f"{len(L)} inputs co-spent, no foreign inputs, exactly "
                         f"one spendable output ({fmt_sats(spend[0].value)}) "
                         "owned by this wallet."),
            inference=("An observer concludes all inputs belonged to one "
                       "controller and that the single output is the merge "
                       "target — your largest UTXO grows and everything feeds "
                       "from it."),
            action=("Before broadcast: split into fewer/no inputs, or add a "
                    "payment so it isn't a pure merge. After broadcast, expect "
                    "the consolidated coin to be permanently linked."),
            limits=("Consolidation is sometimes the right trade-off (fee vs "
                    "privacy); this tool cannot weigh that for you."),
        )
    return _na("consolidation", "not a multi-input single-owned-output merge")


def rule_self_transfer(ntx: NormalizedTx, consolidation_fired: bool,
                      degraded_output: bool) -> Finding:
    spend = ntx.spendable_outputs
    all_owned = spend and all(
        any(f == ntx.fingerprint for f, _, _ in o.derivations) for o in spend)
    if all_owned and not consolidation_fired and not degraded_output:
        return Finding(
            rule="self-transfer", kind="neutral",
            impact=None, confidence=None,
            observation=(f"All {len(spend)} spendable output(s) return to this "
                         "wallet; consolidation didn't fire."),
            inference=("An observer sees a reshuffle inside one wallet; change "
                       "heuristics don't apply because there is no payment to "
                       "contrast against."),
            action=("Every output returns to this wallet; change heuristics "
                    "don't apply."),
            limits=("Internal moves still create linkage between the inputs "
                    "and every output (see common-input-linkage)."),
        )
    return _na("self-transfer", "outputs go somewhere outside this wallet")


def rule_changeless(ntx: NormalizedTx, degraded_output: bool) -> Finding:
    has_change = any(o.role == "change" for o in ntx.outputs)
    has_external = any(o.role == "external" and o.script_type != "op_return"
                       and o.value > 0 for o in ntx.outputs)
    if not has_change and has_external and not degraded_output:
        return Finding(
            rule="changeless", kind="neutral",
            impact=None, confidence=None,
            observation=("No change output, at least one external output."),
            inference=("An observer can't run change-location heuristics at "
                       "all — change heuristics have nothing to find."),
            action=("Changeless coin selection is usually the strongest "
                    "pattern available. Nothing to change."),
            limits=("Not a privacy claim: exact-amount selection itself "
                    "narrows which coins you hold."),
        )
    return _na("changeless", "there is a change output, or no external payment")


def rule_small_input(ntx: NormalizedTx) -> Finding:
    L = ntx.linkable
    small = [i for i in L if i.value < SMALL_UTXO]
    if small and len(L) >= 2:
        desc = "; ".join(ntx.describe_input(i) for i in small)
        return Finding(
            rule="small-input", kind="warning",
            impact="medium", confidence="possible",
            observation=(f"{desc} — below {SMALL_UTXO:,} sats — co-spent with "
                         f"{len(L) - len(small)} other linkable input(s)."),
            inference=("Tiny coins dragged along look like dust-following or "
                       "forced coin selection; observers weight them as a "
                       "linkage hint."),
            action=("Rebuild with coin control: sweep dust separately or omit "
                    "it from this payment."),
            limits=("You may be cleaning dust on purpose; the tool can't know "
                    "your intent."),
        )
    return _na("small-input", f"no linkable input below {SMALL_UTXO:,} sats")


# ---------------------------------------------------------------------------
# Change heuristics — each produces a guess: one spendable output, or none
# ---------------------------------------------------------------------------

def change_gate(ntx: NormalizedTx):
    """Gate: exactly one change output, ≥1 external output, and no other roles
    among the spendable outputs. Otherwise 'not applicable'. No output-count
    restriction — batched payments are covered."""
    ch = [o for o in ntx.outputs if o.role == "change"]
    ext = [o for o in ntx.spendable_outputs if o.role == "external"]
    if len(ch) != 1:
        return None, "no unique change output to compare against"
    if not ext:
        return None, "no external payment output"
    others = [o for o in ntx.spendable_outputs
              if o.role not in ("change", "external")]
    if others:
        return None, ("spendable outputs carry other roles "
                      f"({sorted({o.role for o in others})})")
    return ch[0], None


def guess_round_payment(ntx: NormalizedTx) -> ChangeGuess:
    ch, why = change_gate(ntx)
    if ch is None:
        return ChangeGuess("round-payment", None, False, why)
    spend = ntx.spendable_outputs
    r = {o.index: roundness(o.value) for o in spend}
    lowest = min(r.values())
    tied = [idx for idx, v in r.items() if v == lowest]
    if len(tied) != 1:
        return ChangeGuess("round-payment", None,
                           note="tie at the lowest roundness -> no guess")
    guess = tied[0]
    strong = all(v >= STRONG_ROUND for k, v in r.items() if k != guess)
    return ChangeGuess("round-payment", guess, strong,
                       note=f"roundness={r}, lowest={lowest}")


def guess_script_type_match(ntx: NormalizedTx) -> ChangeGuess:
    ch, why = change_gate(ntx)
    if ch is None:
        return ChangeGuess("script-type-match", None, False, why)
    types = {i.script_type for i in ntx.inputs}
    if len(types) != 1:
        return ChangeGuess("script-type-match", None, False,
                           note="inputs don't share one type")
    t = next(iter(types))
    if t == "unknown":
        return ChangeGuess("script-type-match", None, False,
                           note="'unknown' is never evidence")
    matches = [o for o in ntx.spendable_outputs if o.script_type == t]
    if len(matches) == 1:
        return ChangeGuess("script-type-match", matches[0].index, False,
                           note=f"only spendable output of type {t}")
    return ChangeGuess("script-type-match", None, False,
                       note=f"{len(matches)} outputs share input type {t}")


def guess_optimal_change(ntx: NormalizedTx) -> ChangeGuess:
    if not OPTIMAL_CHANGE_ENABLED:
        return ChangeGuess("optimal-change", None, False, "disabled")
    ch, why = change_gate(ntx)
    if ch is None:
        return ChangeGuess("optimal-change", None, False, why)
    if len(ntx.inputs) < 2:
        return ChangeGuess("optimal-change", None, False, "fewer than 2 inputs")
    m = min(i.value for i in ntx.inputs)
    below = [o for o in ntx.spendable_outputs if o.value < m]
    if len(below) == 1:
        return ChangeGuess("optimal-change", below[0].index, False,
                           note=f"exactly one spendable output < min input {m}")
    return ChangeGuess("optimal-change", None, False,
                       note=f"{len(below)} outputs below min input {m}")


CHANGE_GUESSERS = (guess_round_payment, guess_script_type_match,
                   guess_optimal_change)


# ---------------------------------------------------------------------------
# Combined change verdict — the algorithm from §6, verbatim structure
# ---------------------------------------------------------------------------

def change_verdict(ntx: NormalizedTx, guesses: List[ChangeGuess],
                   true_change_idx: Optional[int],
                   guess_only: bool = False) -> List[Finding]:
    """Compare each observer guess with what the wallet actually did."""
    findings = []
    for g in guesses:
        if g.guess is None:
            findings.append(_na(g.rule, g.note or "no guess"))
            continue
        obs = f"Observer guess: output {g.guess} is change. {g.note}"
        if guess_only:
            findings.append(Finding(
                rule=g.rule, kind="neutral", impact=None, confidence=None,
                observation=obs + " — labeled: observer guess, unverifiable.",
                inference="An observer would place your change at that output.",
                action="Cannot be verified against wallet truth in degraded mode.",
                limits="No output carried your fingerprint; truth comparison impossible.",
            ))
            continue
        if g.guess == true_change_idx:
            findings.append(Finding(
                rule=g.rule, kind="warning", impact=None, confidence=None,
                observation=obs,
                inference="The observer's guess about your change is CORRECT.",
                action="", limits="",
            ))
        else:
            findings.append(Finding(
                rule=g.rule, kind="favorable", impact=None, confidence=None,
                observation=obs,
                inference=("The observer's guess is WRONG: actual change is "
                           f"output {true_change_idx}."),
                action="", limits="",
            ))

    matches = [g for g in guesses if g.guess is not None and g.guess == true_change_idx]
    conflicts = [g for g in guesses
                 if g.guess is not None and g.guess != true_change_idx]

    if not any(f.applicable for f in findings):
        gate_why = next((g.note for g in guesses if g.note), "gate not met")
        return [_na("change-verdict", gate_why)]

    if guess_only:
        # Degraded mode: no truth to compare — every applicable guess stays neutral.
        out = [f for f in findings if f.applicable]
        out.insert(0, Finding(
            rule="change-verdict", kind="neutral", impact=None, confidence=None,
            observation=("Degraded mode: no output carries your fingerprint, so "
                         "change-guess accuracy is unverifiable."),
            inference="Observer change heuristics ran blind; this tool cannot score them.",
            action="Add output derivations in your wallet export, if possible.",
            limits="Missing metadata is not evidence; severity is never lowered by it.",
        ))
        return out

    if matches:
        kind = "warning"
        confident = (len(matches) >= 2 or any(g.strong for g in matches)) and not conflicts
        confidence = "likely" if confident else "possible"
        caveat = " Signals conflict." if conflicts else ""
        impact = "high" if confidence == "likely" else "medium"
        return [Finding(
            rule="change-verdict", kind=kind, impact=impact,
            confidence=confidence,
            observation=(f"{len(matches)} change heuristic(s) correctly "
                         f"identified output {true_change_idx} as change."
                         + caveat),
            inference=("An observer weighing these signals independently would "
                       "find your change output." + caveat),
            action=("Make change less guessable: vary fee so change isn't the "
                    "only odd amount, match script types deliberately, or use "
                    "changeless selection if one exists."),
            limits=("Only round-payment ever marks a guess 'strong'; the "
                    "combined verdict reflects independent-signal weighting, "
                    "never certainty."),
        )]
    if conflicts:
        return [Finding(
            rule="change-verdict", kind="favorable", impact=None,
            confidence="likely",
            observation=(f"{len(conflicts)} heuristic(s) pointed at the wrong "
                         "output; none matched the real change."),
            inference="An observer would likely misidentify your change.",
            action="Keep this shape; nothing to change.",
            limits="Conflicting signals lower observer confidence, never severity.",
        )]
    return [Finding(
        rule="change-verdict", kind="neutral", impact=None, confidence=None,
        observation="No change heuristic produced a guess.",
        inference="The transaction is ambiguous under these checks.",
        action="Nothing indicated.",
        limits="Ambiguity here is not proof of privacy.",
    )]


# ---------------------------------------------------------------------------
# Top-level
# ---------------------------------------------------------------------------

def analyze(ntx: NormalizedTx) -> "AnalysisResultLike":
    """Pure entry point: NormalizedTx -> findings (§4)."""
    findings: List[Finding] = []
    degraded_output = not outputs_carry_fingerprint(ntx)

    # --- input & reuse rules (truth-checked in every mode) ------------------
    findings.append(rule_common_input_linkage(ntx))
    findings.append(rule_foreign_input(ntx))
    findings.append(rule_input_address_reuse(ntx))
    findings.append(rule_sender_reuse(ntx))
    findings.append(rule_recipient_reuse(ntx))
    cons = rule_consolidation(ntx)
    findings.append(cons)
    consolidation_fired = cons.kind == "warning" and cons.applicable
    if ntx.degraded or degraded_output:
        st = _na("self-transfer",
                 "disabled in degraded mode (output ownership unverifiable)")
    else:
        st = rule_self_transfer(ntx, consolidation_fired, degraded_output)
    findings.append(st)
    if ntx.degraded or degraded_output:
        cl = _na("changeless",
                 "disabled in degraded mode (can't tell changeless payment "
                 "from missing output metadata)")
    else:
        cl = rule_changeless(ntx, degraded_output)
    findings.append(cl)
    findings.append(rule_small_input(ntx))

    # --- change heuristics ---------------------------------------------------
    guesses = [fn(ntx) for fn in CHANGE_GUESSERS]
    if ntx.degraded:
        verdict_findings = change_verdict(ntx, guesses, None, guess_only=True)
    else:
        ch, why = change_gate(ntx)
        if ch is None:
            verdict_findings = [_na("round-payment", why),
                                _na("script-type-match", why),
                                _na("optimal-change", why),
                                _na("change-verdict", why)]
        else:
            verdict_findings = change_verdict(ntx, guesses, ch.index)
    findings.extend(verdict_findings)

    return AnalysisResultLike(findings=findings,
                              degraded_output=degraded_output,
                              guesses=guesses)


class AnalysisResultLike:
    """Lightweight container so the report layer can count checks honestly."""

    def __init__(self, findings, degraded_output, guesses):
        self.findings = findings
        self.degraded_output = degraded_output
        self.guesses = guesses

    @property
    def warnings(self):
        return [f for f in self.findings if f.kind == "warning" and f.applicable]

    @property
    def favorables(self):
        return [f for f in self.findings if f.kind == "favorable" and f.applicable]

    @property
    def neutrals(self):
        return [f for f in self.findings if f.kind == "neutral" and f.applicable]

    @property
    def not_applicable(self):
        return [f for f in self.findings if not f.applicable]

    def by_rule(self, rule: str) -> Optional[Finding]:
        for f in self.findings:
            if f.rule == rule:
                return f
        return None

    def counts(self) -> dict:
        applied = len(RULE_CATALOG) - len([
            f for f in self.not_applicable if f.rule in RULE_CATALOG])
        na_rules = {f.rule for f in self.not_applicable if f.rule in RULE_CATALOG}
        # rules that emitted an NA finding count as not applied
        na = len(na_rules) + (len(RULE_CATALOG) - len({f.rule for f in self.findings
                                                      if f.rule in RULE_CATALOG}
                                                     | na_rules))
        return {
            "total": len(RULE_CATALOG),
            "applied": applied,
            "not_applicable": len(RULE_CATALOG) - applied,
            "unavailable": 4,  # §9 items needing wallet history / network data
        }
