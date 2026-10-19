-- Days are carsharing_core::Day values: whole days since 1970-01-01 (UTC).
-- Nothing in this schema can hold a location, a route, a search or a link (spec §9).

CREATE TABLE installs (
    id uuid PRIMARY KEY,
    token_hash bytea NOT NULL UNIQUE CHECK (length(token_hash) = 32),
    first_seen integer NOT NULL,
    decided_votes integer NOT NULL DEFAULT 0 CHECK (decided_votes >= 0),
    matching_votes integer NOT NULL DEFAULT 0
        CHECK (matching_votes >= 0 AND matching_votes <= decided_votes)
);

-- The current shared price of each offer.
CREATE TABLE offers (
    id text PRIMARY KEY,
    provider_id text NOT NULL,
    name text NOT NULL,
    cards jsonb NOT NULL,
    confirmed_on integer,
    locked boolean NOT NULL DEFAULT false
);

-- A price report targets an existing offer, or proposes a new offer of a provider (spec §5.1);
-- an accepted proposal gets its offer_id when the offer is created.
CREATE TABLE reports (
    id uuid PRIMARY KEY,
    offer_id text REFERENCES offers (id),
    proposed_provider text,
    proposed_name text,
    reporter uuid NOT NULL REFERENCES installs (id),
    reporter_weight integer NOT NULL CHECK (reporter_weight BETWEEN 0 AND 10000),
    cards jsonb NOT NULL,
    created_on integer NOT NULL,
    status text NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'accepted', 'superseded', 'reverted')),
    decided_on integer,
    CHECK (offer_id IS NOT NULL OR (proposed_provider IS NOT NULL AND proposed_name IS NOT NULL))
);
CREATE INDEX reports_pending_by_offer ON reports (offer_id, created_on) WHERE status = 'pending';
CREATE INDEX reports_pending_proposals ON reports (proposed_provider, lower(proposed_name))
    WHERE status = 'pending' AND offer_id IS NULL;

-- Votes in the order they were cast; the weight is the voter's weight at that moment.
CREATE TABLE votes (
    seq bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    report_id uuid NOT NULL REFERENCES reports (id),
    install_id uuid NOT NULL REFERENCES installs (id),
    answer text NOT NULL CHECK (answer IN ('yes', 'no')),
    weight integer NOT NULL CHECK (weight BETWEEN 0 AND 10000)
);
CREATE INDEX votes_by_report ON votes (report_id, seq);

-- Every value an offer has had, newest last, so the owner can revert (spec §7.4). Rows are
-- never deleted; a revert marks the row it undoes.
CREATE TABLE price_history (
    seq bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    offer_id text NOT NULL REFERENCES offers (id),
    cards jsonb NOT NULL,
    report_id uuid REFERENCES reports (id), -- NULL: from the seed catalogue
    from_day integer NOT NULL,
    reverted boolean NOT NULL DEFAULT false
);
CREATE INDEX price_history_by_offer ON price_history (offer_id, seq);
