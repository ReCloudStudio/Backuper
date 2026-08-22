CREATE TABLE IF NOT EXISTS jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_id TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    archive_key TEXT,
    error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_jobs_rule_id ON jobs(rule_id);
CREATE INDEX IF NOT EXISTS idx_jobs_started_at ON jobs(started_at);
