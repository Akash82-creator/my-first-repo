"""Design-choice constants (LeakCheck v4 spec §6). Not facts — decisions."""

# Rounding ladder, sats: first unit that divides an amount defines its
# "roundness". Lower on the ladder = rounder = stronger observer signal.
ROUND_LADDER = [1_000_000, 100_000, 10_000, 1_000]

# A payment guess is "strong" only if every other spendable output has
# roundness at least this coarse (i.e. no other output is near-round too).
STRONG_ROUND = 100_000

# Inputs below this value (sats) co-spent with other linkable inputs are a
# dust-adjacent linkage signal.
SMALL_UTXO = 1_000

# Distinct scriptPubKeys in the linkable set L at or above this count raise
# common-input-linkage impact from medium to high.
LINKAGE_HIGH_THRESHOLD = 3
