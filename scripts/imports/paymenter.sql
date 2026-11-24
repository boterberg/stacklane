CREATE DATABASE paymenter;
GRANT ALL ON paymenter.* TO 'import'@'%';
USE paymenter;
CREATE TABLE users (id BIGINT PRIMARY KEY, first_name VARCHAR(255), last_name VARCHAR(255), email VARCHAR(255), role_id BIGINT NULL, password VARCHAR(255), created_at TIMESTAMP NULL, updated_at TIMESTAMP NULL);
CREATE TABLE properties (id BIGINT PRIMARY KEY, custom_property_id BIGINT NULL, name VARCHAR(255), `key` VARCHAR(255), value TEXT, model_type VARCHAR(255), model_id BIGINT);
CREATE TABLE extensions (id BIGINT PRIMARY KEY, name VARCHAR(255), extension VARCHAR(255), type VARCHAR(255), enabled TINYINT(1));
CREATE TABLE products (id BIGINT PRIMARY KEY, category_id BIGINT, name VARCHAR(255), slug VARCHAR(255), description TEXT, server_id BIGINT NULL);
CREATE TABLE plans (id BIGINT PRIMARY KEY, name VARCHAR(255), priceable_type VARCHAR(255), priceable_id BIGINT, type ENUM('free','one-time','recurring'), billing_period INT NULL, billing_unit ENUM('hour','day','week','month','year') NULL);
CREATE TABLE prices (id BIGINT PRIMARY KEY, price DECIMAL(17,2), setup_fee DECIMAL(17,2), currency_code VARCHAR(3), plan_id BIGINT);
CREATE TABLE services (id BIGINT PRIMARY KEY, status VARCHAR(255), product_id BIGINT NULL, user_id BIGINT, currency_code VARCHAR(3), quantity INT, price DECIMAL(17,2), plan_id BIGINT NULL, expires_at DATETIME NULL, label VARCHAR(255) NULL, created_at TIMESTAMP NULL, updated_at TIMESTAMP NULL);
CREATE TABLE invoices (id BIGINT PRIMARY KEY, number VARCHAR(255), status VARCHAR(255), due_at DATE NULL, currency_code VARCHAR(3), user_id BIGINT, created_at TIMESTAMP NULL, updated_at TIMESTAMP NULL);
CREATE TABLE invoice_items (id BIGINT PRIMARY KEY, invoice_id BIGINT, price DECIMAL(17,2), quantity INT, description VARCHAR(255), reference_type VARCHAR(255) NULL, reference_id BIGINT NULL);
CREATE TABLE tickets (id BIGINT PRIMARY KEY, subject VARCHAR(255), status VARCHAR(255), priority VARCHAR(255), department VARCHAR(255) NULL, user_id BIGINT, created_at TIMESTAMP NULL, updated_at TIMESTAMP NULL);
CREATE TABLE ticket_messages (id BIGINT PRIMARY KEY, ticket_id BIGINT, user_id BIGINT, message TEXT, created_at TIMESTAMP NULL);

INSERT INTO users VALUES
  (1, 'Beheer', 'Paymenter', 'admin@hostbedrijf.nl', 1, '$2y$10$.roOaRRm4v6PYPq5KWYRbeWSdnanPUEsVatLchGpXCwzNXfzfuR9G', '2024-01-01 10:00:00', NULL),
  (2, 'Lotte', 'de Graaf', 'lotte@degraaf.nl', NULL, '$2y$10$.roOaRRm4v6PYPq5KWYRbeWSdnanPUEsVatLchGpXCwzNXfzfuR9G', '2025-03-02 09:00:00', NULL),
  (3, 'Bram', 'Jacobs', 'bram@jacobs-it.be', NULL, '$2y$10$.roOaRRm4v6PYPq5KWYRbeWSdnanPUEsVatLchGpXCwzNXfzfuR9G', '2025-06-10 12:00:00', NULL);
INSERT INTO properties VALUES
  (1, 1, 'Bedrijf', 'company_name', 'De Graaf Design', 'App\\Models\\User', 2),
  (2, 2, 'Land', 'country', 'NL', 'App\\Models\\User', 2),
  (3, 3, 'Plaats', 'city', 'Utrecht', 'App\\Models\\User', 2),
  (4, 4, 'Adres', 'address', 'Oudegracht 12', 'App\\Models\\User', 2),
  (5, 5, 'Postcode', 'zip', '3511 AB', 'App\\Models\\User', 2),
  (6, 2, 'Land', 'country', 'BE', 'App\\Models\\User', 3);
INSERT INTO extensions VALUES (1, 'Pterodactyl', 'Pterodactyl', 'server', 1), (2, 'Proxmox', 'Proxmox', 'server', 1);
INSERT INTO products VALUES
  (1, 1, 'Minecraft 6 GB', 'minecraft-6-gb', 'Paper, Fabric of Forge', 1),
  (2, 1, 'Cloud VPS 2', 'cloud-vps-2', '2 vCPU, 4 GB RAM', 2);
INSERT INTO plans VALUES
  (1, 'Maandelijks', 'App\\Models\\Product', 1, 'recurring', 1, 'month'),
  (2, 'Per kwartaal', 'App\\Models\\Product', 1, 'recurring', 3, 'month'),
  (3, 'Jaarlijks', 'App\\Models\\Product', 2, 'recurring', 1, 'year');
INSERT INTO prices VALUES (1, 15.00, 0, 'EUR', 1), (2, 42.00, 0, 'EUR', 2), (3, 240.00, 0, 'EUR', 3);
INSERT INTO services VALUES
  (1, 'active', 1, 2, 'EUR', 1, 42.00, 2, '2027-01-15 00:00:00', 'Survival server', '2025-04-15 10:00:00', NULL),
  (2, 'suspended', 2, 3, 'EUR', 1, 240.00, 3, '2026-11-01 00:00:00', NULL, '2025-11-01 10:00:00', NULL),
  (3, 'cancelled', 1, 3, 'EUR', 1, 15.00, 1, '2025-08-01 00:00:00', NULL, '2025-07-01 10:00:00', NULL);
INSERT INTO invoices VALUES
  (1, 'INV-2026-0041', 'paid', '2026-10-29', 'EUR', 2, '2026-10-15 08:00:00', NULL),
  (2, 'INV-2026-0052', 'pending', '2026-11-15', 'EUR', 3, '2026-11-01 08:00:00', NULL),
  (3, 'INV-2026-0060', 'draft', NULL, 'EUR', 3, '2026-11-20 08:00:00', NULL);
INSERT INTO invoice_items VALUES
  (1, 1, 42.00, 1, 'Minecraft 6 GB (Per kwartaal)', 'App\\Models\\Service', 1),
  (2, 2, 240.00, 1, 'Cloud VPS 2 (Jaarlijks)', 'App\\Models\\Service', 2),
  (3, 3, 15.00, 1, 'Concept', NULL, NULL);
INSERT INTO tickets VALUES
  (1, 'VPS opgeschort?', 'replied', 'high', 'Facturatie', 3, '2026-11-02 09:00:00', '2026-11-02 11:00:00'),
  (2, 'Modpack installeren', 'closed', 'normal', NULL, 2, '2026-05-01 18:00:00', '2026-05-02 09:00:00');
INSERT INTO ticket_messages VALUES
  (1, 1, 3, 'Mijn VPS is opgeschort, maar ik heb betaald?', '2026-11-02 09:00:00'),
  (2, 1, 1, 'Je betaling is net binnengekomen, de VPS draait weer.', '2026-11-02 11:00:00'),
  (3, 2, 2, 'Hoe zet ik een modpack op mijn server?', '2026-05-01 18:00:00'),
  (4, 2, 1, 'Via Bestanden kun je de map uploaden.', '2026-05-02 09:00:00');
