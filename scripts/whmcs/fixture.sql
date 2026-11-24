CREATE TABLE tblcurrencies (id INT PRIMARY KEY, code VARCHAR(5), prefix VARCHAR(10), `default` TINYINT(1));
CREATE TABLE tblclients (
  id INT PRIMARY KEY, firstname TEXT, lastname TEXT, companyname TEXT, email TEXT, address1 TEXT, address2 TEXT,
  city TEXT, state TEXT, postcode TEXT, country TEXT, phonenumber TEXT, currency INT, status ENUM('Active','Inactive','Closed'),
  datecreated DATE
);
CREATE TABLE tblusers (id INT PRIMARY KEY, email TEXT, password TEXT);
CREATE TABLE tblusers_clients (id INT PRIMARY KEY, auth_user_id INT, client_id INT, owner TINYINT(1));
CREATE TABLE tblproducts (id INT PRIMARY KEY, type TEXT, gid INT, name TEXT, description TEXT, hidden TINYINT(1), retired TINYINT(1), servertype TEXT);
CREATE TABLE tblpricing (id INT PRIMARY KEY, type VARCHAR(20), currency INT, relid INT, monthly DECIMAL(16,2), quarterly DECIMAL(16,2), semiannually DECIMAL(16,2), annually DECIMAL(16,2));
CREATE TABLE tblhosting (
  id INT PRIMARY KEY, userid INT, orderid INT, packageid INT, server INT, regdate DATE, domain TEXT, paymentmethod TEXT,
  firstpaymentamount DECIMAL(16,2), amount DECIMAL(16,2), billingcycle TEXT, nextduedate DATE,
  domainstatus ENUM('Pending','Active','Suspended','Terminated','Cancelled','Fraud','Completed'), username TEXT, dedicatedip TEXT
);
CREATE TABLE tblinvoices (
  id INT PRIMARY KEY, userid INT, invoicenum TEXT, date DATE, duedate DATE, datepaid DATETIME, subtotal DECIMAL(16,2),
  tax DECIMAL(16,2), tax2 DECIMAL(16,2), total DECIMAL(16,2), taxrate DECIMAL(10,3), status TEXT
);
CREATE TABLE tblinvoiceitems (id INT PRIMARY KEY, invoiceid INT, userid INT, type TEXT, relid INT, description TEXT, amount DECIMAL(16,2));
CREATE TABLE tblticketdepartments (id INT PRIMARY KEY, name TEXT);
CREATE TABLE tbltickets (id INT PRIMARY KEY, tid TEXT, did INT, userid INT, name TEXT, email TEXT, title TEXT, message TEXT, status TEXT, urgency TEXT, date DATETIME, lastreply DATETIME);
CREATE TABLE tblticketreplies (id INT PRIMARY KEY, tid INT, userid INT, name TEXT, email TEXT, date DATETIME, message TEXT, admin TEXT);
CREATE TABLE tblaccounts (id INT PRIMARY KEY, userid INT, invoiceid INT, amountin DECIMAL(16,2), date DATETIME);

INSERT INTO tblcurrencies VALUES (1, 'EUR', '€', 1);
INSERT INTO tblclients VALUES
  (1, 'Marieke', 'Pieters', 'Pieters Media', 'marieke@pietersmedia.nl', 'Kerkstraat 4', '', 'Gouda', 'Zuid-Holland', '2801 AB', 'NL', '+31.182123456', 1, 'Active', '2023-02-14'),
  (2, 'Tom', 'Vermeulen', '', 'TOM@vermeulen.be', 'Meir 10', 'bus 3', 'Antwerpen', '', '2000', 'BE', '', 1, 'Active', '2024-08-01'),
  (3, 'Geen', 'Mail', '', 'geen-mail', '', '', '', '', '', 'NL', '', 1, 'Closed', '2022-01-01');
INSERT INTO tblusers VALUES (10, 'marieke@pietersmedia.nl', '$2y$10$rXZwYIQDvAPrZvFiyQh2SuaYSooGyY5AxSawbKGrp95qQafKh6Pgm'), (11, 'tom@vermeulen.be', '$2y$10$rXZwYIQDvAPrZvFiyQh2SuaYSooGyY5AxSawbKGrp95qQafKh6Pgm');
INSERT INTO tblusers_clients VALUES (1, 10, 1, 1), (2, 11, 2, 1);
INSERT INTO tblproducts VALUES
  (1, 'other', 1, 'Minecraft 4 GB', 'Paper of Spigot', 0, 0, 'pterodactyl'),
  (2, 'server', 2, 'VPS Medium', '2 vCPU, 4 GB', 0, 0, 'proxmox'),
  (3, 'hostingaccount', 3, 'Webhosting Plus', '', 0, 0, 'cpanel'),
  (4, 'other', 1, 'Oud pakket', '', 1, 1, '');
INSERT INTO tblpricing VALUES
  (1, 'product', 1, 1, 12.50, 36.00, -1, -1), (2, 'product', 1, 2, 18.00, -1, -1, 190.00),
  (3, 'product', 1, 3, -1, -1, -1, 60.00), (4, 'product', 1, 4, 5.00, -1, -1, -1);
INSERT INTO tblhosting VALUES
  (1, 1, 1, 1, 1, '2024-01-10', 'mc.pietersmedia.nl', 'mollie', 12.50, 12.50, 'Monthly', '2026-12-10', 'Active', '', '185.10.1.20'),
  (2, 1, 2, 2, 2, '2023-06-01', 'vps1.pietersmedia.nl', 'mollie', 190.00, 190.00, 'Annually', '2027-06-01', 'Suspended', 'root', '185.10.1.21'),
  (3, 2, 3, 3, 3, '2024-08-01', 'vermeulen.be', 'banktransfer', 110.00, 110.00, 'Biennially', '2026-08-01', 'Active', 'vermeule', ''),
  (4, 2, 4, 4, 0, '2022-01-01', '', 'banktransfer', 5.00, 5.00, 'Monthly', '0000-00-00', 'Terminated', '', ''),
  (5, 3, 5, 1, 1, '2022-01-01', '', 'banktransfer', 12.50, 12.50, 'Monthly', '2022-02-01', 'Cancelled', '', '');
INSERT INTO tblinvoices VALUES
  (1001, 1, '2026-0001', '2026-11-10', '2026-11-24', '2026-11-11 09:15:00', 10.33, 2.17, 0, 12.50, 21.000, 'Paid'),
  (1002, 1, '2026-0002', '2026-11-20', '2026-12-04', '0000-00-00 00:00:00', 157.02, 32.98, 0, 190.00, 21.000, 'Unpaid'),
  (1003, 2, '', '2024-08-01', '2024-08-15', '2024-08-02 10:00:00', 110.00, 0, 0, 110.00, 0.000, 'Paid');
INSERT INTO tblinvoiceitems VALUES
  (1, 1001, 1, 'Hosting', 1, 'Minecraft 4 GB - mc.pietersmedia.nl (10/11/2026 - 09/12/2026)', 12.50),
  (2, 1002, 1, 'Hosting', 2, 'VPS Medium (01/06/2027 - 31/05/2028)', 190.00),
  (3, 1003, 2, 'Hosting', 3, 'Webhosting Plus - vermeulen.be', 110.00);
INSERT INTO tblticketdepartments VALUES (1, 'Technische support'), (2, 'Facturatie');
INSERT INTO tbltickets VALUES
  (1, '482913', 1, 1, '', '', 'Server start niet na update', 'Hoi,\r\n\r\nNa de update naar 1.21 start mijn server niet meer.\r\nKunnen jullie kijken?', 'Answered', 'High', '2026-11-18 20:11:00', '2026-11-19 08:30:00'),
  (2, '553201', 2, 2, '', '', 'Factuur op bedrijfsnaam', 'Kan de factuur op naam van mijn bedrijf? <b>Graag</b>', 'Closed', 'Low', '2024-08-03 11:00:00', '2024-08-03 14:00:00'),
  (3, '600100', 1, 0, 'Gast', 'gast@voorbeeld.nl', 'Vraag vooraf', 'Hebben jullie ook servers in Belgie?', 'Open', 'Medium', '2026-11-20 10:00:00', '2026-11-20 10:00:00');
INSERT INTO tblticketreplies VALUES
  (1, 1, 0, '', '', '2026-11-19 08:30:00', 'Hoi Marieke,\r\n\r\nDe plugin EssentialsX was niet compatibel. We hebben hem bijgewerkt.', 'Sam'),
  (2, 2, 0, '', '', '2024-08-03 13:00:00', 'Dat is aangepast.', 'Sam'),
  (3, 2, 2, '', '', '2024-08-03 14:00:00', 'Top, dank je!', '');
INSERT INTO tblaccounts VALUES (1, 1, 1001, 12.50, '2026-11-11 09:15:00'), (2, 2, 1003, 110.00, '2024-08-02 10:00:00');
