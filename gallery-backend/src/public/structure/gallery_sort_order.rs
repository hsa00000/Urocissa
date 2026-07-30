use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum GallerySortOrder {
    #[default]
    Descending,
    Ascending,
    Random,
}

impl FromStr for GallerySortOrder {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "descending" => Ok(Self::Descending),
            "ascending" => Ok(Self::Ascending),
            "random" => Ok(Self::Random),
            _ => Err("sort must be descending, ascending, or random"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GallerySortOrder;

    #[test]
    fn query_values_are_strictly_parsed() {
        assert_eq!(
            "descending".parse::<GallerySortOrder>().unwrap(),
            GallerySortOrder::Descending
        );
        assert_eq!(
            "ascending".parse::<GallerySortOrder>().unwrap(),
            GallerySortOrder::Ascending
        );
        assert_eq!(
            "random".parse::<GallerySortOrder>().unwrap(),
            GallerySortOrder::Random
        );
        assert!("newest".parse::<GallerySortOrder>().is_err());
    }
}
