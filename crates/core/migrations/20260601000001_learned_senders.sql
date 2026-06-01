-- Sender column on detection_events lets us learn from the user's
-- Confirm / Reject decisions in the inbox.
ALTER TABLE detection_events ADD COLUMN sender TEXT;
CREATE INDEX idx_detection_events_sender ON detection_events(sender);

-- One row per sender. Decision flips between 'allow' (Confirmed at least
-- once) and 'block' (Rejected). The scan loop checks block early to skip
-- the LLM round-trip.
CREATE TABLE learned_senders (
    sender      TEXT NOT NULL PRIMARY KEY,
    decision    TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE INDEX idx_learned_senders_decision ON learned_senders(decision);
