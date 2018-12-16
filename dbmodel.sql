
-- ------
-- BGA framework: © Gregory Isabelli <gisabelli@boardgamearena.com> & Emmanuel Colin <ecolin@boardgamearena.com>
-- GuerreMagiciens implementation : © <Your name here> <Your email address here>
-- 
-- This code has been produced on the BGA studio platform for use on http://boardgamearena.com.
-- See http://en.boardgamearena.com/#!doc/Studio for more information.
-- -----

-- dbmodel.sql

-- This is the file where you are describing the database schema of your game
-- Basically, you just have to export from PhpMyAdmin your table structure and copy/paste
-- this export here.
-- Note that the database itself and the standard tables ("global", "stats", "gamelog" and "player") are
-- already created and must not be created here

-- Note: The database schema is created from this file when the game starts. If you modify this file,
--       you have to restart a game to see your changes in database.

-- Example 1: create a standard "card" table to be used with the "Deck" tools (see example game "hearts"):

-- CREATE TABLE IF NOT EXISTS 'card' (
--   'card_id' int(10) unsigned NOT NULL AUTO_INCREMENT,
--   'card_type' varchar(16) NOT NULL,
--   'card_type_arg' int(11) NOT NULL,
--   'card_location' varchar(16) NOT NULL,
--   'card_location_arg' int(11) NOT NULL,
--   PRIMARY KEY ('card_id')
-- ) ENGINE=InnoDB DEFAULT CHARSET=utf8 AUTO_INCREMENT=1 ;


-- Example 2: add a custom field to the standard "player" table
-- ALTER TABLE 'player' ADD 'player_my_custom_field' INT UNSIGNED NOT NULL DEFAULT '0';

ALTER TABLE player ADD player_money SMALLINT UNSIGNED NOT NULL DEFAULT 50;
ALTER TABLE player ADD town_criers_expense SMALLINT UNSIGNED NOT NULL DEFAULT 0;
ALTER TABLE player ADD laboratories TINYINT UNSIGNED NOT NULL DEFAULT 3;

ALTER TABLE player ADD toratsa_in_stock TINYINT UNSIGNED NOT NULL DEFAULT 0;
ALTER TABLE player ADD xephis_in_stock TINYINT UNSIGNED NOT NULL DEFAULT 0;
ALTER TABLE player ADD yaboul_in_stock TINYINT UNSIGNED NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS talismans_in_stock (
  talisman_name VARCHAR(8) NOT NULL,
  talismans_amount TINYINT UNSIGNED NOT NULL,
  talisman_code TINYINT UNSIGNED NOT NULL
);

CREATE TABLE IF NOT EXISTS fanatics (
  player_id INTEGER NOT NULL,
  fanatics_names VARCHAR(6) NOT NULL,
  fanatics_strength TINYINT NOT NULL,
  in_hall boolean NOT NULL
);
