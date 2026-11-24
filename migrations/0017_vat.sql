ALTER TABLE users ADD COLUMN tax_id_verified_at TIMESTAMPTZ;

ALTER TABLE invoices
    ADD COLUMN vat_scheme TEXT NOT NULL DEFAULT 'domestic'
        CHECK (vat_scheme IN ('domestic', 'oss', 'reverse_charge', 'outside_eu')),
    ADD COLUMN vat_country CHAR(2),
    ADD COLUMN customer_vat_number TEXT;

CREATE INDEX invoices_vat_scheme_issued ON invoices (vat_scheme, issued_at);
