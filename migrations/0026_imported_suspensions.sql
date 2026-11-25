UPDATE services SET suspension_reason = 'payment'
WHERE status = 'suspended' AND import_ref IS NOT NULL AND suspension_reason = 'admin'
  AND updated_at < created_at + interval '1 minute';

UPDATE subscriptions s SET suspended_for_payment = TRUE
WHERE s.status = 'suspended' AND s.import_ref IS NOT NULL
  AND EXISTS (SELECT 1 FROM services v WHERE v.subscription_id = s.id AND v.suspension_reason = 'payment');
