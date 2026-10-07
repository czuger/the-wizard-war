-- Hidden playing style of each computer-controlled alchemist; earlier games keep the balanced one.
ALTER TABLE players ADD COLUMN temperament TEXT NOT NULL DEFAULT 'balanced';
ALTER TABLE players ADD COLUMN temperament_seed INTEGER NOT NULL DEFAULT 0;
