-- Runs once, when the Postgres volume is first created.
-- Integration tests use this database (TEST_DATABASE_URL), never the dev one.
CREATE DATABASE voclara_test OWNER voclara;
