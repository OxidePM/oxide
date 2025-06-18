
CREATE TABLE store_obj (
    id   INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    path TEXT UNIQUE NOT NULL,
    hash TEXT NOT NULL -- the hash used to compute the path
);

CREATE TABLE ref (
    referrer  INTEGER NOT NULL,
    reference INTEGER NOT NULL,
    PRIMARY KEY (referrer, reference),
    FOREIGN KEY (referrer) REFERENCES store_obj(id) ON DELETE CASCADE,
    FOREIGN KEY (reference) REFERENCES store_obj(id) ON DELETE RESTRICT
);

CREATE INDEX index_referrer ON ref(referrer);
CREATE INDEX index_reference ON ref(reference);

CREATE TRIGGER delete_self_ref BEFORE DELETE ON store_obj 
BEGIN
  DELETE FROM ref WHERE referrer = old.id AND reference = old.id;
END;

----------------------------------------------------------------------------- 

CREATE TABLE realisation (
    id       INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    eq_class TEXT NOT NULL,
    out      TEXT NOT NULL, -- symbolic output, usually "out"
    obj      INTEGER NOT NULL, -- the realised store object
    UNIQUE (eq_class, out, obj),
    FOREIGN KEY (obj) REFERENCES store_obj(id) ON DELETE CASCADE
);

CREATE INDEX index_realisation_class ON realisation(eq_class, out);
CREATE INDEX index_realisation_obj ON realisation(obj);

CREATE TABLE realisation_ref (
    referrer  INTEGER NOT NULL,
    reference INTEGER NOT NULL,
    FOREIGN KEY (referrer) REFERENCES realisation(id) ON DELETE CASCADE,
    FOREIGN KEY (reference) REFERENCES realisation(id) ON DELETE RESTRICT
);


CREATE INDEX index_realisation_referrer on realisation_ref(referrer);
CREATE INDEX index_realisation_reference on realisation_ref(reference);

CREATE TRIGGER delete_self_refs_realisation BEFORE DELETE ON store_obj
BEGIN
  DELETE FROM realisation_ref WHERE reference IN (
    SELECT id FROM realisation WHERE obj = old.id
  );
END;

