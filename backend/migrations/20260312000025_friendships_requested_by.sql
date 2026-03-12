-- Add requested_by to friendships so we can distinguish sender from receiver.
ALTER TABLE friendships ADD COLUMN IF NOT EXISTS requested_by UUID REFERENCES users(id);

-- Backfill existing rows: we can't know the original requester, so leave NULL.
-- NULL rows will be excluded from the incoming-requests list query.
