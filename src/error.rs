//! Application errors.

use thiserror::Error;

use crate::domain::{ConversionError, PurchaseError};

/// Everything that can go wrong while handling a request.
///
/// The `Display` text is shown to players, so it is written in French.
#[derive(Debug, Error)]
pub enum AppError {
    /// The database failed.
    #[error("Erreur de base de données.")]
    Database(#[from] sqlx::Error),
    /// The database schema could not be migrated.
    #[error("Impossible de mettre à jour la base de données.")]
    Migration(#[from] sqlx::migrate::MigrateError),
    /// The folder holding the database file could not be created.
    #[error("Impossible de créer le dossier de la base de données.")]
    DatabaseFolder(#[from] std::io::Error),
    /// A page could not be rendered.
    #[error("Impossible d'afficher la page.")]
    Template(#[from] askama::Error),
    /// A stored value does not match any known variant.
    #[error("Donnée invalide en base ({column} = {value}).")]
    CorruptData {
        /// The column holding the value.
        column: &'static str,
        /// The value that could not be read.
        value: String,
    },
    /// No game has this id.
    #[error("Cette partie n'existe pas.")]
    GameNotFound,
    /// No player of the game has this token.
    #[error("Ce lien de joueur n'est pas valide.")]
    PlayerNotFound,
    /// The action does not belong to the current phase.
    #[error("Cette action n'est pas possible à ce moment de la partie.")]
    WrongPhase,
    /// Another player must act first.
    #[error("Ce n'est pas votre tour.")]
    NotYourTurn,
    /// The player has already acted in this phase.
    #[error("Vous avez déjà joué cette phase.")]
    AlreadyDone,
    /// The fanatic pawns cannot be dealt to this number of players.
    #[error("Ce nombre de joueurs n'est pas pris en charge.")]
    InvalidPlayerCount,
    /// The player name is empty, too long or taken by a computer-controlled alchemist.
    #[error(
        "Votre nom doit compter de 1 à 30 caractères et ne pas être celui d'un alchimiste joué par l'ordinateur."
    )]
    InvalidPlayerName,
    /// The number of turns is out of range.
    #[error("Le nombre de tours doit être compris entre 1 et 30.")]
    InvalidTurnCount,
    /// The district is not one of the city's six.
    #[error("Choisissez un des six quartiers de la cité.")]
    InvalidDistrict,
    /// The level of the computer-controlled alchemists is unknown.
    #[error("Niveau des alchimistes inconnu.")]
    InvalidAiLevel,
    /// The fanatic is not one the player can send to the hall.
    #[error("Ce fanatique ne vous appartient pas.")]
    FanaticNotAvailable,
    /// The expense is negative or not a multiple of the smallest banknote.
    #[error("La dépense doit être un multiple positif de 100 écus.")]
    InvalidExpense,
    /// The player cannot pay that much.
    #[error("Vous n'avez pas assez d'écus.")]
    InsufficientFunds,
    /// The player did not take as many items as required.
    #[error("Vous devez prendre exactement {expected} objet(s) magique(s).")]
    WrongProductionCount {
        /// The number of items the player must take.
        expected: i64,
    },
    /// The supply does not hold the requested items.
    #[error("La réserve ne contient pas assez de ces objets.")]
    SupplyExhausted,
    /// The conversion breaks a rule of phase II.1.
    #[error(transparent)]
    Conversion(#[from] ConversionError),
    /// The player tried to pass while a conversion is possible.
    #[error("Vous devez choisir une conversion.")]
    ConversionRequired,
    /// The rebel moves are missing, duplicated or not along a path to a village.
    #[error("Chaque pion rebelle doit avancer d'un village le long d'un chemin.")]
    InvalidRebelMove,
    /// A rebels pawn may only attack a pawn standing on its village.
    #[error("Les rebelles ne peuvent attaquer qu'un colporteur ou une caravane de leur village.")]
    InvalidFightTarget,
    /// A pawn order is not allowed.
    #[error(
        "Déplacement impossible : un colporteur avance d'un lieu, une caravane d'un ou deux, le long des chemins, et un pion arrêté ne bouge pas."
    )]
    InvalidPawnMove,
    /// A pawn leaving the city must carry an item from the player's stock.
    #[error("Un pion qui quitte la ville doit emporter un objet de votre stock.")]
    CargoRequired,
    /// The pawn cannot sell where it stands.
    #[error(
        "Ce pion ne peut pas vendre ici : le lieu doit acheter cet objet et, sauf un monastère, n'avoir rien acheté d'autre ce tour-ci."
    )]
    InvalidSale,
    /// The player does not have this item in stock.
    #[error("Vous n'avez pas cet objet en stock.")]
    NotInStock,
    /// The purchase breaks a rule of phase VII.
    #[error(transparent)]
    Purchase(#[from] PurchaseError),
}

impl AppError {
    /// Returns `true` for server-side failures, as opposed to invalid player requests.
    pub fn is_internal(&self) -> bool {
        matches!(
            self,
            AppError::Database(_)
                | AppError::Migration(_)
                | AppError::DatabaseFolder(_)
                | AppError::Template(_)
                | AppError::CorruptData { .. }
        )
    }
}
