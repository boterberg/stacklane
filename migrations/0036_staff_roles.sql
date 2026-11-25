ALTER TABLE users DROP CONSTRAINT users_role_check;
ALTER TABLE users ADD CONSTRAINT users_role_check CHECK (role IN ('customer', 'staff', 'admin'));
ALTER TABLE users ADD COLUMN staff_permissions TEXT[] NOT NULL DEFAULT '{}';
