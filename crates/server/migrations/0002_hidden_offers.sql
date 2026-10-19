-- The owner can hide an offer (one the crowd created by abuse, say): it leaves the catalogue and
-- takes no reports, votes or confirmations, but its rows and history stay (decision 46).
ALTER TABLE offers ADD COLUMN hidden boolean NOT NULL DEFAULT false;
