CREATE TABLE departments (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    position INTEGER NOT NULL DEFAULT 0
);

INSERT INTO departments (id, name, position) VALUES
    (gen_random_uuid(), 'VPS', 1),
    (gen_random_uuid(), 'Webhosting', 2),
    (gen_random_uuid(), 'Gameservers', 3),
    (gen_random_uuid(), 'Algemeen', 4),
    (gen_random_uuid(), 'Sales', 5),
    (gen_random_uuid(), 'Domeinen', 6);

CREATE TABLE tickets (
    id UUID PRIMARY KEY,
    number BIGSERIAL UNIQUE,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    department_id UUID NOT NULL REFERENCES departments (id),
    subject TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('open', 'answered', 'awaiting_reply', 'in_progress', 'closed')),
    last_reply_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX tickets_user_id_idx ON tickets (user_id);

CREATE TABLE ticket_messages (
    id UUID PRIMARY KEY,
    ticket_id UUID NOT NULL REFERENCES tickets (id) ON DELETE CASCADE,
    author_user_id UUID REFERENCES users (id) ON DELETE SET NULL,
    is_staff BOOLEAN NOT NULL DEFAULT FALSE,
    body_html TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
