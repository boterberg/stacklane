UPDATE users
SET tax_id_verified_at = NULL
WHERE tax_id_verified_at IS NOT NULL
  AND upper(left(regexp_replace(tax_id, '[^A-Za-z0-9]', '', 'g'), 2)) ~ '^[A-Z]{2}$'
  AND upper(left(regexp_replace(tax_id, '[^A-Za-z0-9]', '', 'g'), 2))
      <> CASE country WHEN 'GR' THEN 'EL' ELSE country::text END;
