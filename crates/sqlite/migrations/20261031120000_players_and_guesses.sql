-- Players, the links from identity tokens to them, and their Guesses (spec #26, "Schema").
-- Times are integer nanoseconds since the Unix epoch, so they keep the clock's precision and sort as numbers.

CREATE TABLE players (
    id TEXT PRIMARY KEY NOT NULL,
    -- Changed in the same transaction as a resolution, never summed from the Guesses.
    score INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
) STRICT;

-- No foreign key to players: the link is written before the Player, in its own transaction (ADR 0008).
CREATE TABLE identity_links (
    token_hash BLOB PRIMARY KEY NOT NULL CHECK (length(token_hash) = 32),
    player_id TEXT NOT NULL
) STRICT;

CREATE TABLE guesses (
    id TEXT PRIMARY KEY NOT NULL,
    player_id TEXT NOT NULL REFERENCES players (id),
    direction TEXT NOT NULL CHECK (direction IN ('up', 'down')),
    placed_at INTEGER NOT NULL,
    start_price_cents INTEGER NOT NULL CHECK (start_price_cents >= 0),
    start_received_at INTEGER NOT NULL,
    start_source TEXT NOT NULL CHECK (start_source IN ('bitstamp')),
    start_trade_id INTEGER,
    state TEXT NOT NULL CHECK (state IN ('open', 'resolved')),
    end_price_cents INTEGER CHECK (end_price_cents >= 0),
    end_received_at INTEGER,
    end_source TEXT CHECK (end_source IN ('bitstamp')),
    end_trade_id INTEGER,
    outcome TEXT CHECK (outcome IN ('correct', 'wrong')),
    resolved_at INTEGER,
    CHECK (
        (state = 'open'
            AND end_price_cents IS NULL AND end_received_at IS NULL AND end_source IS NULL
            AND end_trade_id IS NULL AND outcome IS NULL AND resolved_at IS NULL)
        OR (state = 'resolved'
            AND end_price_cents IS NOT NULL AND end_received_at IS NOT NULL AND end_source IS NOT NULL
            AND outcome IS NOT NULL AND resolved_at IS NOT NULL)
    )
) STRICT;

-- One Open guess per Player.
CREATE UNIQUE INDEX guesses_one_open_per_player ON guesses (player_id) WHERE state = 'open';

-- Serves a Player's most recent Resolved guess.
CREATE INDEX guesses_by_player_and_resolved_at ON guesses (player_id, resolved_at);
