"""PSBT fixture factory for tests and demo samples (synthetic signet data).

Builds *real* PSBT bytes with embit — v0 and v2, base64 and binary, unsigned /
partially signed / fully signed, Taproot inputs, PayJoin-style foreign inputs,
OP_RETURN outputs, and degraded-mode variants with derivations stripped.

No network access. Amounts/keys are synthetic; they mirror signet shapes
(BIP86 p2tr, BIP84 p2wpkh, coin type 1' = testnet/signet).
"""

from __future__ import annotations

import hashlib
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

from embit.bip32 import HDKey
from embit.descriptor import Descriptor
from embit.hdpath import HDPath
from embit.networks import NETWORKS
from embit.psbt import DerivationPath, PSBT
from embit.script import Script
from embit.transaction import (Transaction, TransactionInput,
                               TransactionOutput)

SIGNET = NETWORKS["signet"]

# Two distinct "wallets" so we can build owned / foreign / unattributed inputs.
SEED_A = hashlib.sha512(b"leakcheck-fixture-wallet-A").digest()[:64]
SEED_B = hashlib.sha512(b"leakcheck-fixture-wallet-B").digest()[:64]


def wallet_a() -> HDKey:
    return HDKey.from_seed(SEED_A, network=SIGNET)


def wallet_b() -> HDKey:
    return HDKey.from_seed(SEED_B, network=SIGNET)


def fp_int(root: HDKey) -> int:
    return int.from_bytes(root.my_fingerprint, "big")


# ---------------------------------------------------------------------------
# descriptor helpers
# ---------------------------------------------------------------------------

def wpkh_desc(root: HDKey, account=0, net="signet"):
    xprv = root.key.at(HDPath("m/84h/1h/%dh" % account), do_not_check_private=True)
    d = Descriptor.from_string(
        f"wpkh([{xprv.my_fingerprint.hex()}/84h/1h/{account}h]"
        f"{xprv.to_public().to_base58()}/{{0,1}}/*)", network=net)
    d.private = xprv
    return d


def tr_desc(root: HDKey, account=0, net="signet"):
    xprv = root.key.at(HDPath("m/86h/1h/%dh" % account), do_not_check_private=True)
    d = Descriptor.from_string(
        f"tr([{xprv.my_fingerprint.hex()}/86h/1h/{account}h]"
        f"{xprv.to_public().to_base58()}/*)", network=net)
    d.private = xprv
    return d


def sh_wpkh_desc(root: HDKey, account=0, net="signet"):
    xprv = root.key.at(HDPath("m/49h/1h/%dh" % account), do_not_check_private=True)
    d = Descriptor.from_string(
        f"sh(wpkh([{xprv.my_fingerprint.hex()}/49h/1h/{account}h]"
        f"{xprv.to_public().to_base58()}/{{0,1}}/*))", network=net)
    d.private = xprv
    return d


# ---------------------------------------------------------------------------
# UTXO + PSBT assembly
# ---------------------------------------------------------------------------

class Coin:
    """A synthetic prevout: descriptor-derived address + value."""

    def __init__(self, desc: Descriptor, change: bool, index: int, value: int):
        self.desc = desc
        self.change = change
        self.index = index
        self.value = value
        spk, _ = desc.derive(change * 1000000 + index).script_pubkey()
        self.script_pubkey = spk
        derived = desc.derive(change * 1000000 + index)
        k = derived.keys[0]
        if getattr(desc, "is_taproot", False):
            # key-path spend: the registered pubkey is the TWEAKED output key
            self.pubkey = k.taproot_output_key([])
        else:
            self.pubkey = k.key
        txid = hashlib.sha256(
            (f"coin:{desc.origin or ''}:{change}:{index}:{value}").encode()).digest()
        self.txid = txid           # internal byte order (as vin.txid)
        self.vout = 0

    @property
    def witness_utxo(self):
        return TransactionOutput(self.value, self.script_pubkey)

    def add_to_input(self, inp: "object") -> None:
        """Attach witness_utxo + BIP32 derivation (or taproot fields)."""
        inp.witness_utxo = self.witness_utxo
        path = self.desc.derive(self.index + (1000000 if self.change else 0)).full_derivation_path
        der = DerivationPath(self.desc.origin.fingerprint, list(path))
        if getattr(self.desc, "is_taproot", False):
            inp.taproot_bip32_derivations[self.pubkey] = ([], der)
            inp.taproot_internal_key = self.desc.derive(
                self.index + (1000000 if self.change else 0)).keys[0].key
        else:
            inp.bip32_derivations[self.pubkey] = der


def _coin_from_spec(spec, wallets):
    """spec = ('wpkh'|'tr'|'sh-wpkh', wallet_key, change, index, value)"""
    kind, wname, change, index, value = spec
    root = wallets[wname]
    desc = {"wpkh": wpkh_desc, "tr": tr_desc, "sh-wpkh": sh_wpkh_desc}[kind](root)
    return Coin(desc, change, index, value)


def build_psbt(input_specs, output_specs, *, version=None, wallets=("A", "B"),
               locktime=0, tx_version=2, sequences=None, sign=False,
               partial_sign=False, strip_out_derivations=False,
               strip_in_derivations=(), op_return_pushes=None):
    """Assemble a PSBT.

    input_specs:  list of (kind, wallet, change, index, value); wallet in "A"/"B"/None
                  (None => no derivation on that input: unattributed)
    output_specs: list of (kind, wallet, change, index, value) or ("op_return", value)
    version:      None => v0, 2 => v2
    """
    roots = {"A": wallet_a(), "B": wallet_b()}
    coins = []
    for spec in input_specs:
        kind, wname, change, index, value = spec
        if wname is None:
            # unattributed: random-looking script with no derivation
            h = hashlib.sha256(("unattr:%d:%d" % (index, value)).encode()).digest()
            spk = Script(bytes.fromhex("0014" + h[:20].hex()))
            c = _coin_from_spec((kind, "B", change, index, value), roots)
            c.script_pubkey = spk          # observer sees this script
            c.pubkey = None                # no derivation attached
            coins.append(c)
        else:
            coins.append(_coin_from_spec(spec, roots))

    vins, vouts = [], []
    for i, c in enumerate(coins):
        seq = (sequences[i] if sequences else 0xfffffffd)
        vins.append(TransactionInput(txid=c.txid, vout=c.vout, sequence=seq))
    for spec in output_specs:
        if spec[0] == "op_return":
            payload = (op_return_pushes or {}).get(output_specs.index(spec), b"\x6a\x24lease")
            vouts.append(TransactionOutput(value=spec[1], script_pubkey=Script(payload)))
            continue
        kind, wname, change, index, value = spec
        root = roots[wname]
        desc = {"wpkh": wpkh_desc, "tr": tr_desc, "sh-wpkh": sh_wpkh_desc}[kind](root)
        derived = desc.derive(index + (1000000 if change else 0))
        spk, _ = derived.script_pubkey()
        vouts.append(TransactionOutput(value=value, script_pubkey=spk))

    tx = Transaction(version=tx_version, vin=vins, vout=vouts, locktime=locktime)
    psbt = PSBT(tx)
    if version == 2:
        psbt.version = 2

    for i, c in enumerate(coins):
        if c.pubkey is not None:
            c.add_to_input(psbt.inputs[i])
        else:
            psbt.inputs[i].witness_utxo = c.witness_utxo

    if not strip_out_derivations:
        for j, spec in enumerate(output_specs):
            if spec[0] == "op_return":
                continue
            kind, wname, change, index, value = spec
            root = roots[wname]
            desc = {"wpkh": wpkh_desc, "tr": tr_desc, "sh-wpkh": sh_wpkh_desc}[kind](root)
            pos = index + (1000000 if change else 0)
            derived = desc.derive(pos)
            k = derived.keys[0]
            path = derived.full_derivation_path
            der = DerivationPath(desc.origin.fingerprint, list(path))
            if getattr(desc, "is_taproot", False):
                pub = k.taproot_output_key([])
                psbt.outputs[j].taproot_bip32_derivations[pub] = ([], der)
            else:
                psbt.outputs[j].bip32_derivations[k.key] = der

    for i in strip_in_derivations:
        inp = psbt.inputs[i]
        inp.bip32_derivations.clear()
        inp.taproot_bip32_derivations.clear()
        inp.taproot_internal_key = None

    if sign or partial_sign:
        for i, c in enumerate(coins):
            if c.pubkey is None:
                continue
            try:
                psbt.inputs[i].partial_sigs[c.pubkey] = _fake_sig(c, psbt, i)
            except Exception:
                pass
        if sign:
            for i, c in enumerate(coins):
                if c.pubkey is None:
                    continue
                sig = psbt.inputs[i].partial_sigs.get(c.pubkey)
                if sig is not None and not getattr(c.desc, "is_taproot", False):
                    from embit.script import p2wpkh_scriptcode
                    ss = (Script(b"\x16\x00\x14" + c.pubkey.hash160())
                          if len(c.pubkey.serialize()) == 33 else None)
                    if c.desc and str(c.desc).startswith("sh"):
                        inner = Script(bytes.fromhex("0014") + c.pubkey.hash160())
                        rs = Script(b"\xa9\x14" + inner.sha256() + b"\x87")
                        ss = rs
                    if ss is not None:
                        psbt.inputs[i].final_scriptsig = Script(
                            bytes(sig) + b"\x21" + bytes(c.pubkey.serialize()))
                elif sig is not None:
                    from embit.script import Witness
                    psbt.inputs[i].final_scriptwitness = Witness([bytes(sig)])

    return psbt


def _fake_sig(coin, psbt, i):
    """Deterministic 71-byte blob standing in for a signature (fixtures only)."""
    return hashlib.sha256(b"sig:" + coin.txid + str(i).encode()).digest() + b"\x01" * 39


def to_base64(psbt: PSBT) -> str:
    import base64
    return base64.b64encode(psbt.serialize()).decode()


def to_binary(psbt: PSBT) -> bytes:
    return psbt.serialize()


# ---------------------------------------------------------------------------
# Named scenario builders used by tests AND demo samples
# ---------------------------------------------------------------------------

V0 = 0xFFFFFFFD  # RBF-signaling sequence

def demo_a():
    """PSBT A: 3 coins incl. a small one, paying a round amount, change back.
    More linkable under these checks."""
    return build_psbt(
        input_specs=[
            ("wpkh", "A", False, 3, 5_000_000),
            ("wpkh", "A", True, 0, 3_000_000),
            ("wpkh", "A", False, 4, 900),          # small input (<1000)
        ],
        output_specs=[
            ("wpkh", "B", False, 12, 5_000_000),   # round payment 0.05
            ("wpkh", "A", True, 7, 2_999_500),     # non-round change
        ],
    )


def demo_b():
    """PSBT B: the same payment from ONE coin via coin control. Linkage
    findings disappear; round-payment stays with its 'limited' card."""
    return build_psbt(
        input_specs=[("wpkh", "A", False, 3, 5_000_500)],
        output_specs=[
            ("wpkh", "B", False, 12, 5_000_000),   # same round payment
            ("wpkh", "A", True, 7, 500 - 0),       # tiny odd change below min input
        ],
    )


def payjoin():
    """Owned + foreign inputs (receiver added one), receiver's own input."""
    return build_psbt(
        input_specs=[
            ("wpkh", "A", False, 3, 2_000_000),
            ("wpkh", "A", True, 1, 1_500_000),
            ("wpkh", "B", False, 8, 600_000),      # foreign (receiver) input
        ],
        output_specs=[
            ("wpkh", "B", False, 12, 2_000_000),   # payment
            ("wpkh", "A", True, 9, 2_099_000),     # our change
        ],
    )


def taproot_tx():
    return build_psbt(
        input_specs=[("tr", "A", False, 2, 3_000_000)],
        output_specs=[
            ("tr", "B", False, 5, 2_000_000),
            ("tr", "A", True, 3, 999_500),
        ],
    )


if __name__ == "__main__":
    here = os.path.dirname(os.path.abspath(__file__))
    out = os.path.join(here, "..", "..", "samples")
    os.makedirs(out, exist_ok=True)
    for name, fn in [("demo_a", demo_a), ("demo_b", demo_b),
                     ("payjoin", payjoin), ("taproot", taproot_tx)]:
        p = fn()
        with open(os.path.join(out, name + ".psbt"), "wb") as f:
            f.write(to_binary(p))
        with open(os.path.join(out, name + ".base64.txt"), "w") as f:
            f.write(to_base64(p))
        print("wrote", name)
