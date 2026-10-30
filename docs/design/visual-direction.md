# Visual direction

How Hoof & Claw looks and feels.
Domain terms follow [GLOSSARY.md](../../GLOSSARY.md).

## Tone

A polished product with a dry wink.
The craft is restrained; the name, the tagline and a few small touches (hoof print, claw mark) carry the wit.
The bull/bear theme is an accent, never something a player has to decode to play.

Tagline: "One minute. One call. Hoof or claw."

## Palette

Dark-only, with no theme toggle.
The palette, "tracks at night", takes its colours from animal tracks in earth, bone and horn.

| Name    | Hex       | Role                                                   |
| ------- | --------- | ------------------------------------------------------ |
| Loam    | `#1C1A16` | Background                                             |
| Turf    | `#2A2721` | Raised surfaces: guess card, buttons                   |
| Bone    | `#EDE6D6` | Primary text, price, wordmark                          |
| Flint   | `#8F887A` | Muted text: rules line, "Price delayed"                |
| Pasture | `#7DBB6E` | Up                                                     |
| Rust    | `#D9573B` | Down                                                   |
| Horn    | `#D4B26A` | Logo ampersand and keyboard focus ring only            |

Pasture and Rust mean Up and Down and nothing else.
Every use of Pasture or Rust comes with an arrow and a label, so players with red/green colour blindness can still play.

## Typography

Geist Sans is the only typeface.
The price, the countdown and the score use tabular figures so the digits don't shift as they change.

## Logo

Three concepts go to mockups; bull and bear heads are out.

1. **Ampersand mark.** The "&" of the wordmark is drawn from a cloven hoof print and a claw slash.
2. **Split mark.** A compact mark with a hoof print on a Pasture half and three claw slashes on a Rust half, beside a plain wordmark.
   It doubles as the favicon.
3. **Tracks.** A hoof print and claw marks crossing a single price line.

Only the chosen logo enters the repo, as SVG.

## Layout

A single, mobile-first, centred column:

```
┌──────────────────────────────┐
│ [logo]              Score 3  │
│                              │
│        $67,412.18            │
│        BTC/USD               │
│                              │
│   [ Up ↑ ]     [ Down ↓ ]    │
│   Guess where BTC goes in    │
│   the next minute.           │
│   Right +1, wrong −1.        │
│                              │
│  ┌────────────────────────┐  │
│  │      guess card        │  │
│  └────────────────────────┘  │
└──────────────────────────────┘
```

Score and price are always visible.
The Up and Down buttons carry plain-word labels, with the bull/bear motif only as an icon accent.
They are disabled while a guess is open.
The rules line is muted and always present; there is no modal or onboarding flow.

## Guess card

### Open guess

A ring drains over 60 seconds, with the remaining seconds in its centre.
The card shows the live difference from the Start price, for example `+$12.40 ↑`, coloured by whether the guess would currently be Correct.
The label stays factual, without "you're winning" copy.

If 60 seconds have passed and the price hasn't changed, the ring becomes a slow pulse labelled "Waiting for the price to move…".

### Resolved guess

The open-guess card turns into a result card in place, with no toast or full-screen takeover.
It shows the Start price and End price, the outcome (Correct or Wrong) and the score changing by +1 or −1.
A stamp marks which way the price went: a hoof print in Pasture for up, a claw mark in Rust for down.
The card stays until the next guess.

A player returning after their guess resolved while they were away sees the same result card.

## Motion

The stamp landing on the result card is the only choreographed animation, with the score ticking in time with it.
The countdown ring drains linearly; buttons react only to presses.
With `prefers-reduced-motion`, the stamp and score appear without animation.

## Price unavailable

When the price feed fails, the price shows its last known value in Flint, labelled "Price delayed".
The Up and Down buttons stay disabled until the price recovers, so no guess gets a stale Start price.
