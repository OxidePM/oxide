
CREATE TABLE path (
    id   INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    path TEXT UNIQUE NOT NULL,
    hash TEXT NOT NULL
);

CREATE TABLE ref (
    referrer  INTEGER NOT NULL,
    reference INTEGER NOT NULL,
    PRIMARY KEY (referrer, reference),
    FOREIGN KEY (referrer) REFERENCES path(id) ON DELETE CASCADE,
    FOREIGN KEY (reference) REFERENCES path(id) ON DELETE RESTRICT
);

CREATE INDEX index_referrer ON ref(referrer);
CREATE INDEX index_reference ON ref(reference);

CREATE TRIGGER delete_self_ref BEFORE DELETE ON path 
BEGIN
  DELETE FROM ref WHERE referrer = old.id AND reference = old.id;
END;

-- TODO: maybe add table derivation_output

----------------------------------------------------------------------------- 

-- TODO: check below schema

CREATE TABLE realisation (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    drv_path TEXT NOT NULL,
    output_name TEXT NOT NULL, -- symbolic output, usually "out"
    output_path INTEGER NOT NULL,
    FOREIGN KEY (output_path) REFERENCES path(id) ON DELETE CASCADE
);

CREATE INDEX index_realisation_path ON realisation(drv_path, output_name);
CREATE INDEX index_realisation_output_path ON realisation(outputPath);

CREATE TABLE realisation_ref (
    referrer INTEGER NOT NULL,
    reference INTEGER NOT NULL,
    PRIMARY KEY (referrer, reference),
    FOREIGN KEY (referrer) REFERENCES realisation(id) ON DELETE CASCADE,
    foreign key (reference) REFERENCES realisation(id) ON DELETE RESTRICT
);


CREATE INDEX index_realisation_referrer on realisation_ref(referrer);
CREATE INDEX index_realisation_reference on realisation_ref(reference);

CREATE TRIGGER delete_self_refs_realisation BEFORE DELETE ON path 
  BEGIN
    DELETE FROM realisation_ref WHERE reference IN (
      SELECT id FROM realisation WHERE output_path = old.id
    );
  END;

