-- Support tickets: user-submitted bugs, ideas, and improvement requests.
-- Each row mirrors a HubSpot ticket (hubspot_ticket_id stored after creation).

CREATE TABLE support_tickets (
    id                 UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id            UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ticket_type        TEXT        NOT NULL CHECK (ticket_type IN ('bug', 'idea', 'improvement')),
    subject            TEXT        NOT NULL CHECK (char_length(subject) BETWEEN 1 AND 200),
    description        TEXT        NOT NULL CHECK (char_length(description) BETWEEN 1 AND 2000),
    priority           TEXT        NOT NULL DEFAULT 'medium'
                                   CHECK (priority IN ('low', 'medium', 'high', 'urgent')),
    status             TEXT        NOT NULL DEFAULT 'open',
    hubspot_ticket_id  TEXT,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX support_tickets_user_id_idx ON support_tickets (user_id);
CREATE INDEX support_tickets_created_at_idx ON support_tickets (created_at DESC);
