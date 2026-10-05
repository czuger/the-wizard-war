use crate::domain::SiteId;

/// Where a peddler or caravan stands: inside the city walls, or on a site of the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Location {
    /// Inside the city, with its owner's laboratories.
    City,
    /// On a village, castle or monastery.
    Site(SiteId),
}

impl Location {
    /// Returns the value stored in the database: `NULL` for the city.
    pub fn as_db(self) -> Option<i64> {
        match self {
            Location::City => None,
            Location::Site(site) => Some(site.as_db()),
        }
    }

    /// Reads a value stored by [`Location::as_db`].
    ///
    /// # Returns
    /// `None` when the site id is out of range.
    pub fn from_db(value: Option<i64>) -> Option<Location> {
        match value {
            None => Some(Location::City),
            Some(id) => SiteId::from_db(id).map(Location::Site),
        }
    }

    /// The site, unless the pawn is in the city.
    pub fn site(self) -> Option<SiteId> {
        match self {
            Location::City => None,
            Location::Site(site) => Some(site),
        }
    }

    /// Encodes the location for a form field: `city`, or the site id.
    pub fn form_value(self) -> String {
        match self {
            Location::City => "city".to_string(),
            Location::Site(site) => site.0.to_string(),
        }
    }

    /// Parses a value made by [`Location::form_value`].
    pub fn from_form_value(value: &str) -> Option<Location> {
        match value.trim() {
            "city" => Some(Location::City),
            id => id.parse().ok().map(|id| Location::Site(SiteId(id))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for location in [Location::City, Location::Site(SiteId(12))] {
            assert_eq!(Location::from_db(location.as_db()), Some(location));
            assert_eq!(
                Location::from_form_value(&location.form_value()),
                Some(location)
            );
        }
        assert_eq!(Location::from_form_value("north"), None);
    }
}
