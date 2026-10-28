CREATE TABLE samples(v INTEGER);
INSERT INTO samples VALUES(4),(9);
SELECT count(*),sum(v) FROM samples;
