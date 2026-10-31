# Hoof & Claw

A game where players guess whether the BTC/USD price will be higher or lower after one minute.

## Language

### Guessing

**Player**:
A person playing the game, with one Score and at most one Open guess.
_Avoid_: User, account

**Guess**:
A player's prediction that the BTC/USD price will be higher (Up) or lower (Down) than at the moment of guessing.
_Avoid_: Call (except in the tagline), bet, prediction

**Price source**:
The third-party feed the server receives BTC/USD prices from; the server is the only party that observes prices.
_Avoid_: Oracle, ticker, provider

**Price delayed**:
The state in which the server has no live connection to the Price source; the last price stays visible, and no guess can be placed.
_Avoid_: Offline, stale, outage

**Direction**:
The side a guess takes: Up or Down.
_Avoid_: Bull/bear, long/short, hoof/claw (those are visual motifs, not terms)

**Open guess**:
A guess that is not yet resolved; a player has at most one.
It stays open past 60 seconds for as long as the price has not changed.
_Avoid_: Pending, overtime, active bet

**Resolved guess**:
A guess for which at least 60 seconds have passed and the price has changed since it was placed.
_Avoid_: Settled, closed, finished

**Start price**:
The last BTC/USD price the server received before it accepted a guess.
_Avoid_: Entry price, guess price, opening price

**End price**:
The BTC/USD price a guess is resolved against: the first price the server receives from the Start price's Price source, at least 60 seconds after the guess was placed, that differs from the Start price by at least one tick.
_Avoid_: Exit price, resolution price, closing price

**Outcome**:
The result of a resolved guess: Correct if the price moved in the guessed direction, Wrong otherwise.
_Avoid_: Win/loss, hit/miss

### Scoring

**Score**:
A player's running total, starting at 0, +1 per Correct and −1 per Wrong outcome; it can be negative.
_Avoid_: Points, balance
