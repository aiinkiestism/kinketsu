-- v0.1 initial schema for kinketsu.
-- All timestamps are stored as TEXT (RFC 3339); all UUIDs as BLOB (16 bytes).

CREATE TABLE payment_methods (
    id          BLOB    PRIMARY KEY,
    name        TEXT    NOT NULL,
    kind        TEXT    NOT NULL,
    last4       TEXT,
    color       TEXT,
    icon        TEXT,
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL
);

CREATE TABLE categories (
    id          BLOB    PRIMARY KEY,
    name        TEXT    NOT NULL UNIQUE,
    icon        TEXT,
    color       TEXT,
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL
);

CREATE TABLE subscriptions (
    id                  BLOB    PRIMARY KEY,
    name                TEXT    NOT NULL,
    service_icon        TEXT,
    plan                TEXT,
    amount_minor        INTEGER NOT NULL,
    currency            TEXT    NOT NULL,
    billing_cycle       TEXT    NOT NULL,
    next_billing_date   TEXT,
    started_at          TEXT,
    payment_method_id   BLOB    REFERENCES payment_methods(id) ON DELETE SET NULL,
    category_id         BLOB    REFERENCES categories(id)      ON DELETE SET NULL,
    status              TEXT    NOT NULL DEFAULT 'active',
    notes               TEXT,
    created_at          TEXT    NOT NULL,
    updated_at          TEXT    NOT NULL
);

CREATE INDEX idx_subscriptions_status         ON subscriptions(status);
CREATE INDEX idx_subscriptions_next_billing   ON subscriptions(next_billing_date);
CREATE INDEX idx_subscriptions_payment_method ON subscriptions(payment_method_id);
CREATE INDEX idx_subscriptions_category       ON subscriptions(category_id);

CREATE TABLE detection_events (
    id                       BLOB    PRIMARY KEY,
    source                   TEXT    NOT NULL,
    source_ref               TEXT,
    raw_summary              TEXT,
    parsed_payload           TEXT    NOT NULL,
    confidence               REAL    NOT NULL,
    status                   TEXT    NOT NULL DEFAULT 'pending',
    matched_subscription_id  BLOB    REFERENCES subscriptions(id) ON DELETE SET NULL,
    reviewed_at              TEXT,
    created_at               TEXT    NOT NULL
);

CREATE INDEX idx_detection_events_status     ON detection_events(status);
CREATE INDEX idx_detection_events_source_ref ON detection_events(source_ref);

CREATE TABLE exchange_rates (
    base        TEXT    NOT NULL,
    quote       TEXT    NOT NULL,
    rate        REAL    NOT NULL,
    fetched_at  TEXT    NOT NULL,
    PRIMARY KEY (base, quote, fetched_at)
);

CREATE TABLE settings (
    key         TEXT    PRIMARY KEY,
    value       TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL
);
