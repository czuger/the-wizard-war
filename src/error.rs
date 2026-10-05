//! Application errors.

use thiserror::Error;

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
