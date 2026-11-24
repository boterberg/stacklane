ALTER TABLE services ADD COLUMN suspension_reason TEXT
    CHECK (suspension_reason IN ('admin', 'payment', 'ended'));

UPDATE services v SET suspension_reason = CASE
        WHEN s.status = 'cancelled' THEN 'ended'
        WHEN s.suspended_for_payment THEN 'payment'
        ELSE 'admin'
    END
FROM subscriptions s
WHERE v.status = 'suspended' AND s.id = v.subscription_id;

UPDATE services SET suspension_reason = 'admin'
WHERE status = 'suspended' AND suspension_reason IS NULL;
