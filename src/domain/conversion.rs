use thiserror::Error;

use crate::domain::board::site_by_id;
use crate::domain::{Cult, CultMarker, SiteId};

/// One of the three actions a player chooses from in phase II.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conversion {
    /// Place a marker, emblem up, on a site that has none.
    Place {
        /// The site converted.
        site: SiteId,
        /// The cult of the new marker.
        cult: Cult,
    },
    /// Turn a marker over.
    Flip {
        /// The site whose marker is turned.
        site: SiteId,
    },
    /// Replace a face-down marker with one of the rival cult, emblem up, and post rebels there.
    Replace {
        /// The site converted.
        site: SiteId,
    },
}

/// Why a conversion is refused. The texts are shown to players.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ConversionError {
    /// The site is not on the map.
    #[error("Ce lieu n'existe pas.")]
    UnknownSite,
    /// A marker can only be placed on a site without one.
    #[error("Ce lieu porte déjà un marqueur de culte.")]
    AlreadyMarked,
    /// Restriction (a): on the first turn, markers go on villages joined to the board's edge.
    #[error(
        "Au premier tour, un marqueur ne peut aller que sur un village relié au bord du plateau."
    )]
    NotOnEdge,
    /// Restriction (b): later, markers go next to a site that already has one.
    #[error(
        "Un marqueur ne peut aller que sur un lieu relié par un chemin à un lieu qui en porte déjà un."
    )]
    NotNextToMarker,
    /// Flipping or replacing needs a marker.
    #[error("Ce lieu ne porte aucun marqueur de culte.")]
    NoMarker,
    /// Restriction (c): castles and monasteries never change.
    #[error("Les châteaux et les monastères ne changent jamais de culte.")]
    PermanentSite,
    /// Only a face-down marker can be replaced.
    #[error("Seul un marqueur face cachée peut être remplacé.")]
    NotFaceDown,
    /// All the markers of this cult are on the map.
    #[error("Il ne reste plus de marqueur {}.", .0.label())]
    NoMarkerLeft(Cult),
    /// All the rebels pawns are on the map.
    #[error("Tous les pions rebelles sont déjà sur le plateau.")]
    NoRebelLeft,
}

impl Conversion {
    /// The site the conversion acts on.
    pub fn site(&self) -> SiteId {
        match *self {
            Conversion::Place { site, .. }
            | Conversion::Flip { site }
            | Conversion::Replace { site } => site,
        }
    }

    /// Encodes the conversion for a form field, e.g. `place:12:xephis` or `flip:3`.
    pub fn form_value(&self) -> String {
        match *self {
            Conversion::Place { site, cult } => format!("place:{}:{}", site.0, cult.as_db_str()),
            Conversion::Flip { site } => format!("flip:{}", site.0),
            Conversion::Replace { site } => format!("replace:{}", site.0),
        }
    }

    /// Decodes a value made by [`Conversion::form_value`].
    ///
    /// # Returns
    /// `None` when the value is malformed. The site may still not exist.
    pub fn from_form_value(value: &str) -> Option<Conversion> {
        let mut parts = value.trim().split(':');
        let action = parts.next()?;
        let site = SiteId(parts.next()?.parse().ok()?);
        let conversion = match action {
            "place" => Conversion::Place {
                site,
                cult: Cult::from_db_str(parts.next()?)?,
            },
            "flip" => Conversion::Flip { site },
            "replace" => Conversion::Replace { site },
            _ => return None,
        };
        parts.next().is_none().then_some(conversion)
    }

    /// French label of the action, for the form.
    ///
    /// # Arguments
    /// * `before` - the marker on the site before the conversion.
    pub fn label(&self, before: Option<CultMarker>) -> String {
        let place = site_name(self.site());
        match (*self, before) {
            (Conversion::Place { cult, .. }, _) => {
                format!("Placer un marqueur {} sur {place}", cult.label())
            }
            (Conversion::Flip { .. }, Some(marker)) => {
                format!("Retourner le marqueur de {place} ({})", marker.describe())
            }
            (Conversion::Replace { .. }, Some(marker)) => format!(
                "Remplacer le marqueur {} caché de {place} par un marqueur {} et y poster des rebelles",
                marker.cult.label(),
                marker.cult.opposite().label()
            ),
            _ => format!("Convertir {place}"),
        }
    }

    /// French log message telling what `player` did.
    ///
    /// # Arguments
    /// * `after` - the marker on the site once the conversion is done.
    pub fn narrate(&self, player: &str, after: CultMarker) -> String {
        let place = site_name(self.site());
        match self {
            Conversion::Place { .. } => format!(
                "{player} place un marqueur {} sur {place}.",
                after.cult.label()
            ),
            Conversion::Flip { .. } => format!(
                "{player} retourne le marqueur de {place} ({}).",
                after.describe()
            ),
            Conversion::Replace { .. } => format!(
                "{player} convertit {place} au culte de {} : des rebelles s'y soulèvent.",
                after.cult.label()
            ),
        }
    }
}

fn site_name(id: SiteId) -> String {
    site_by_id(id)
        .map(|site| site.describe())
        .unwrap_or_else(|| format!("le lieu {}", id.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_value_round_trip() {
        for conversion in [
            Conversion::Place {
                site: SiteId(12),
                cult: Cult::Yaboul,
            },
            Conversion::Flip { site: SiteId(3) },
            Conversion::Replace { site: SiteId(30) },
        ] {
            assert_eq!(
                Conversion::from_form_value(&conversion.form_value()),
                Some(conversion)
            );
        }
        for bad in [
            "",
            "place:3",
            "flip",
            "flip:x",
            "burn:3",
            "flip:3:4",
            "place:3:toratsa",
        ] {
            assert_eq!(Conversion::from_form_value(bad), None, "{bad}");
        }
    }
}
