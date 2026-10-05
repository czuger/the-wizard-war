use std::collections::BTreeMap;

use crate::domain::board::{SITES, neighbours, site_by_id};
use crate::domain::rules::{MARKERS_PER_CULT, REBEL_COUNT};
use crate::domain::{Conversion, ConversionError, Cult, CultMarker, SiteId};

/// The markers and rebels on the map: everything phase II reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BoardState {
    markers: BTreeMap<SiteId, CultMarker>,
    rebels: Vec<SiteId>,
}

impl BoardState {
    /// Builds the state from the markers by site and the position of each rebels pawn.
    pub fn new(markers: BTreeMap<SiteId, CultMarker>, rebels: Vec<SiteId>) -> BoardState {
        BoardState { markers, rebels }
    }

    /// The marker on `site`, if any.
    pub fn marker(&self, site: SiteId) -> Option<CultMarker> {
        self.markers.get(&site).copied()
    }

    /// Number of rebels pawns on `site`.
    pub fn rebels_on(&self, site: SiteId) -> usize {
        self.rebels.iter().filter(|&&at| at == site).count()
    }

    /// Markers of `cult` not yet on the map.
    pub fn markers_left(&self, cult: Cult) -> i64 {
        let used = self.markers.values().filter(|marker| marker.cult == cult);
        MARKERS_PER_CULT - used.count() as i64
    }

    /// Rebels pawns not yet on the map.
    pub fn rebels_left(&self) -> i64 {
        REBEL_COUNT - self.rebels.len() as i64
    }

    /// Checks a conversion against the rules of phase II.1.
    ///
    /// # Arguments
    /// * `turn` - the current turn, starting at 1.
    ///
    /// # Returns
    /// The marker the site holds once the conversion is done.
    ///
    /// # Errors
    /// The [`ConversionError`] naming the broken rule.
    pub fn validate(
        &self,
        conversion: &Conversion,
        turn: i64,
    ) -> Result<CultMarker, ConversionError> {
        let site = site_by_id(conversion.site()).ok_or(ConversionError::UnknownSite)?;
        let current = self.marker(site.id);
        match *conversion {
            Conversion::Place { cult, .. } => {
                self.check_placement(site.id, turn)?;
                if self.markers_left(cult) <= 0 {
                    return Err(ConversionError::NoMarkerLeft(cult));
                }
                Ok(CultMarker::face_up(cult))
            }
            Conversion::Flip { .. } => {
                let marker = current.ok_or(ConversionError::NoMarker)?;
                if site.kind.is_permanent() {
                    return Err(ConversionError::PermanentSite);
                }
                Ok(marker.flipped())
            }
            Conversion::Replace { .. } => self.check_replacement(current, site.kind.is_permanent()),
        }
    }

    /// Every conversion allowed this turn, sites in id order.
    pub fn legal_conversions(&self, turn: i64) -> Vec<Conversion> {
        SITES
            .iter()
            .flat_map(|site| {
                let id = site.id;
                [
                    Conversion::Place {
                        site: id,
                        cult: Cult::Xephis,
                    },
                    Conversion::Place {
                        site: id,
                        cult: Cult::Yaboul,
                    },
                    Conversion::Flip { site: id },
                    Conversion::Replace { site: id },
                ]
            })
            .filter(|conversion| self.validate(conversion, turn).is_ok())
            .collect()
    }

    /// Restrictions (a) and (b) on where a new marker may go.
    fn check_placement(&self, site: SiteId, turn: i64) -> Result<(), ConversionError> {
        if self.markers.contains_key(&site) {
            return Err(ConversionError::AlreadyMarked);
        }
        if turn <= 1 {
            let on_edge = site_by_id(site).is_some_and(|site| site.on_edge);
            return if on_edge {
                Ok(())
            } else {
                Err(ConversionError::NotOnEdge)
            };
        }
        if neighbours(site)
            .iter()
            .any(|next| self.markers.contains_key(next))
        {
            Ok(())
        } else {
            Err(ConversionError::NotNextToMarker)
        }
    }

    fn check_replacement(
        &self,
        current: Option<CultMarker>,
        permanent: bool,
    ) -> Result<CultMarker, ConversionError> {
        let marker = current.ok_or(ConversionError::NoMarker)?;
        if permanent {
            return Err(ConversionError::PermanentSite);
        }
        if marker.face_up {
            return Err(ConversionError::NotFaceDown);
        }
        let cult = marker.cult.opposite();
        if self.markers_left(cult) <= 0 {
            return Err(ConversionError::NoMarkerLeft(cult));
        }
        if self.rebels_left() <= 0 {
            return Err(ConversionError::NoRebelLeft);
        }
        Ok(CultMarker::face_up(cult))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EDGE_VILLAGE: SiteId = SiteId(1);
    const INLAND_VILLAGE: SiteId = SiteId(11);
    const CASTLE: SiteId = SiteId(10);

    fn board(markers: &[(u8, Cult, bool)], rebels: &[u8]) -> BoardState {
        let markers = markers
            .iter()
            .map(|&(site, cult, face_up)| (SiteId(site), CultMarker { cult, face_up }))
            .collect();
        BoardState::new(markers, rebels.iter().map(|&site| SiteId(site)).collect())
    }

    fn place(site: SiteId, cult: Cult) -> Conversion {
        Conversion::Place { site, cult }
    }

    #[test]
    fn first_turn_markers_go_on_edge_villages() {
        let empty = BoardState::default();
        assert_eq!(
            empty.validate(&place(EDGE_VILLAGE, Cult::Xephis), 1),
            Ok(CultMarker::face_up(Cult::Xephis))
        );
        assert_eq!(
            empty.validate(&place(INLAND_VILLAGE, Cult::Xephis), 1),
            Err(ConversionError::NotOnEdge)
        );
        let edge_count = SITES.iter().filter(|site| site.on_edge).count();
        assert_eq!(empty.legal_conversions(1).len(), edge_count * 2);
    }

    #[test]
    fn later_markers_go_next_to_a_marked_site() {
        let state = board(&[(2, Cult::Yaboul, false)], &[]);
        assert!(
            state
                .validate(&place(INLAND_VILLAGE, Cult::Xephis), 2)
                .is_ok()
        );
        assert_eq!(
            state.validate(&place(EDGE_VILLAGE, Cult::Xephis), 3),
            Ok(CultMarker::face_up(Cult::Xephis)),
            "village 1 is next to village 2"
        );
        assert_eq!(
            state.validate(&place(SiteId(32), Cult::Xephis), 2),
            Err(ConversionError::NotNextToMarker)
        );
        assert_eq!(
            state.validate(&place(SiteId(2), Cult::Xephis), 2),
            Err(ConversionError::AlreadyMarked)
        );
    }

    #[test]
    fn flipping_toggles_village_markers_only() {
        let state = board(&[(3, Cult::Xephis, true), (10, Cult::Yaboul, true)], &[]);
        assert_eq!(
            state.validate(&Conversion::Flip { site: SiteId(3) }, 2),
            Ok(CultMarker {
                cult: Cult::Xephis,
                face_up: false
            })
        );
        assert_eq!(
            state.validate(&Conversion::Flip { site: CASTLE }, 2),
            Err(ConversionError::PermanentSite)
        );
        assert_eq!(
            state.validate(&Conversion::Flip { site: SiteId(4) }, 2),
            Err(ConversionError::NoMarker)
        );
    }

    #[test]
    fn replacing_needs_a_face_down_village_marker_and_a_rebel() {
        let state = board(&[(3, Cult::Xephis, false), (4, Cult::Xephis, true)], &[]);
        assert_eq!(
            state.validate(&Conversion::Replace { site: SiteId(3) }, 2),
            Ok(CultMarker::face_up(Cult::Yaboul))
        );
        assert_eq!(
            state.validate(&Conversion::Replace { site: SiteId(4) }, 2),
            Err(ConversionError::NotFaceDown)
        );
        let crowded = board(&[(3, Cult::Xephis, false)], &[1, 1, 2, 2, 3, 6, 7]);
        assert_eq!(
            crowded.validate(&Conversion::Replace { site: SiteId(3) }, 2),
            Err(ConversionError::NoRebelLeft)
        );
        assert_eq!(crowded.rebels_on(SiteId(1)), 2);
    }

    #[test]
    fn markers_run_out() {
        let markers: Vec<(u8, Cult, bool)> =
            (1..=21).map(|site| (site, Cult::Xephis, true)).collect();
        let state = board(&markers, &[]);
        assert_eq!(state.markers_left(Cult::Xephis), 0);
        assert_eq!(
            state.validate(&place(SiteId(22), Cult::Xephis), 2),
            Err(ConversionError::NoMarkerLeft(Cult::Xephis))
        );
        assert!(state.validate(&place(SiteId(22), Cult::Yaboul), 2).is_ok());
    }

    #[test]
    fn unknown_sites_are_refused() {
        assert_eq!(
            BoardState::default().validate(&Conversion::Flip { site: SiteId(99) }, 1),
            Err(ConversionError::UnknownSite)
        );
    }
}
