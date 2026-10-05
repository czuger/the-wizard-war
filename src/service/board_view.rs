use crate::db::RebelRecord;
use crate::domain::board::{BOARD_HEIGHT, BOARD_WIDTH, SITES, paths, site_by_id};
use crate::domain::rebel_movement::village_destinations;
use crate::domain::{BoardState, Conversion, Cult, CultMarker, Site, SiteId};
use crate::service::city_view::percent;
use crate::service::trade_view::PawnToken;

/// One site of the map as drawn on the page.
#[derive(Debug, Clone)]
pub struct SiteView {
    /// Number shown on the map.
    pub id: u8,
    /// The site's name, kind and zone, shown on hover.
    pub title: String,
    /// Horizontal position, in percent of the map's width.
    pub left: String,
    /// Vertical position, in percent of the map's height.
    pub top: String,
    /// The marker on the site, if any.
    pub marker: Option<CultMarker>,
    /// Number of rebels pawns on the site.
    pub rebels: usize,
    /// Peddlers and caravans on the site.
    pub pawns: Vec<PawnToken>,
}

impl SiteView {
    /// CSS classes drawing the marker: its cult, and whether it is face down.
    pub fn marker_class(&self) -> &'static str {
        match self.marker {
            None => "unmarked",
            Some(CultMarker {
                cult: Cult::Xephis,
                face_up: true,
            }) => "xephis",
            Some(CultMarker {
                cult: Cult::Xephis,
                face_up: false,
            }) => "xephis hidden",
            Some(CultMarker {
                cult: Cult::Yaboul,
                face_up: true,
            }) => "yaboul",
            Some(CultMarker {
                cult: Cult::Yaboul,
                face_up: false,
            }) => "yaboul hidden",
        }
    }

    /// Path of the marker's token image; face-down markers all share the "?" token.
    pub fn marker_image(&self) -> Option<&'static str> {
        let marker = self.marker?;
        Some(match (marker.cult, marker.face_up) {
            (_, false) => "/img/token-cult-hidden.png",
            (Cult::Xephis, true) => "/img/token-cult-xephis.png",
            (Cult::Yaboul, true) => "/img/token-cult-yaboul.png",
        })
    }
}

/// A path of the map, in pixels of `img/board.jpg`.
#[derive(Debug, Clone, Copy)]
pub struct PathView {
    /// Start, horizontal.
    pub x1: u16,
    /// Start, vertical.
    pub y1: u16,
    /// End, horizontal.
    pub x2: u16,
    /// End, vertical.
    pub y2: u16,
}

/// One conversion the viewing player may choose.
#[derive(Debug, Clone)]
pub struct ConversionOption {
    /// Form value, decoded by [`Conversion::from_form_value`].
    pub value: String,
    /// French description.
    pub label: String,
}

/// The conversions of one kind (place, flip or replace).
#[derive(Debug, Clone)]
pub struct ConversionGroup {
    /// French title of the kind.
    pub label: &'static str,
    /// The conversions of this kind, by site.
    pub options: Vec<ConversionOption>,
}

/// A village a rebels pawn may move to.
#[derive(Debug, Clone)]
pub struct DestinationOption {
    /// Id of the village.
    pub site: u8,
    /// Name of the village.
    pub name: &'static str,
}

/// A rebels pawn the viewing player has to move.
#[derive(Debug, Clone)]
pub struct RebelChoice {
    /// Id of the pawn, used in the form field name.
    pub rebel_id: i64,
    /// Name of the village the pawn stands on.
    pub from: &'static str,
    /// Where it may go.
    pub destinations: Vec<DestinationOption>,
}

/// The map as everyone sees it, plus the viewing player's phase II choices.
#[derive(Debug, Clone, Default)]
pub struct BoardView {
    /// Every site, in id order.
    pub sites: Vec<SiteView>,
    /// Every path.
    pub paths: Vec<PathView>,
    /// Conversions the viewing player may choose; empty unless it is their turn to convert.
    pub conversion_groups: Vec<ConversionGroup>,
    /// Pawns the viewing player must move; empty unless it is their turn to move rebels.
    pub rebel_choices: Vec<RebelChoice>,
}

impl BoardView {
    /// Builds the view of the map.
    ///
    /// # Arguments
    /// * `conversion_turn` - the current turn when the viewer must convert, else `None`.
    /// * `must_move_rebels` - whether the viewer must move the rebels.
    /// * `tokens` - the peddlers and caravans on the map.
    pub fn build(
        state: &BoardState,
        rebels: &[RebelRecord],
        conversion_turn: Option<i64>,
        must_move_rebels: bool,
        tokens: &[PawnToken],
    ) -> BoardView {
        BoardView {
            sites: SITES
                .iter()
                .map(|site| site_view(state, site, tokens))
                .collect(),
            paths: paths().filter_map(|(a, b)| path_view(a, b)).collect(),
            conversion_groups: conversion_turn
                .map(|turn| conversion_groups(state, turn))
                .unwrap_or_default(),
            rebel_choices: if must_move_rebels {
                rebel_choices(rebels)
            } else {
                Vec::new()
            },
        }
    }
}

fn site_view(state: &BoardState, site: &Site, tokens: &[PawnToken]) -> SiteView {
    let marker = state.marker(site.id);
    let marker_text = marker.map_or("aucun marqueur".to_string(), CultMarker::describe);
    SiteView {
        id: site.id.0,
        title: format!("{}. {} — {marker_text}", site.id.0, site.describe()),
        left: percent(site.x, BOARD_WIDTH),
        top: percent(site.y, BOARD_HEIGHT),
        marker,
        rebels: state.rebels_on(site.id),
        pawns: tokens
            .iter()
            .filter(|token| token.site == site.id)
            .cloned()
            .collect(),
    }
}

fn path_view(a: SiteId, b: SiteId) -> Option<PathView> {
    let (from, to) = (site_by_id(a)?, site_by_id(b)?);
    Some(PathView {
        x1: from.x,
        y1: from.y,
        x2: to.x,
        y2: to.y,
    })
}

fn conversion_groups(state: &BoardState, turn: i64) -> Vec<ConversionGroup> {
    let legal = state.legal_conversions(turn);
    let group = |label: &'static str, keep: fn(&Conversion) -> bool| ConversionGroup {
        label,
        options: legal
            .iter()
            .filter(|conversion| keep(conversion))
            .map(|conversion| ConversionOption {
                value: conversion.form_value(),
                label: format!(
                    "{}. {}",
                    conversion.site().0,
                    conversion.label(state.marker(conversion.site()))
                ),
            })
            .collect(),
    };
    [
        group("Placer un marqueur", |c| {
            matches!(c, Conversion::Place { .. })
        }),
        group("Retourner un marqueur", |c| {
            matches!(c, Conversion::Flip { .. })
        }),
        group("Remplacer un marqueur caché", |c| {
            matches!(c, Conversion::Replace { .. })
        }),
    ]
    .into_iter()
    .filter(|group| !group.options.is_empty())
    .collect()
}

fn rebel_choices(rebels: &[RebelRecord]) -> Vec<RebelChoice> {
    rebels
        .iter()
        .filter_map(|rebel| {
            let destinations: Vec<DestinationOption> = village_destinations(rebel.site)
                .into_iter()
                .filter_map(site_by_id)
                .map(|site| DestinationOption {
                    site: site.id.0,
                    name: site.name,
                })
                .collect();
            (!destinations.is_empty()).then(|| RebelChoice {
                rebel_id: rebel.id,
                from: site_by_id(rebel.site).map_or("?", |site| site.name),
                destinations,
            })
        })
        .collect()
}
