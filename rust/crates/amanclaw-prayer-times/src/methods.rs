use serde::{Deserialize, Serialize};
use std::fmt;

/// Prayer time calculation method with Fajr/Isha angles or offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CalculationMethod {
    /// Muslim World League — Fajr 18deg, Isha 17deg
    MWL,
    /// Islamic Society of North America — Fajr 15deg, Isha 15deg
    ISNA,
    /// Egyptian General Authority of Survey — Fajr 19.5deg, Isha 17.5deg
    Egyptian,
    /// University of Islamic Sciences, Karachi — Fajr 18deg, Isha 18deg
    Karachi,
    /// Umm Al-Qura University, Makkah — Fajr 18.5deg, Isha 90min after Maghrib
    UmmAlQura,
    /// Jabatan Kemajuan Islam Malaysia — Fajr 20deg, Isha 18deg
    JAKIM,
}

/// Parameters for a calculation method.
pub struct MethodParams {
    /// Fajr angle below horizon in degrees.
    pub fajr_angle: f64,
    /// Isha angle below horizon in degrees. Ignored if `isha_minutes` is Some.
    pub isha_angle: f64,
    /// If Some, Isha is calculated as this many minutes after Maghrib.
    pub isha_minutes: Option<u32>,
}

impl CalculationMethod {
    /// Return the calculation parameters for this method.
    pub fn params(self) -> MethodParams {
        match self {
            Self::MWL => MethodParams {
                fajr_angle: 18.0,
                isha_angle: 17.0,
                isha_minutes: None,
            },
            Self::ISNA => MethodParams {
                fajr_angle: 15.0,
                isha_angle: 15.0,
                isha_minutes: None,
            },
            Self::Egyptian => MethodParams {
                fajr_angle: 19.5,
                isha_angle: 17.5,
                isha_minutes: None,
            },
            Self::Karachi => MethodParams {
                fajr_angle: 18.0,
                isha_angle: 18.0,
                isha_minutes: None,
            },
            Self::UmmAlQura => MethodParams {
                fajr_angle: 18.5,
                isha_angle: 0.0,
                isha_minutes: Some(90),
            },
            Self::JAKIM => MethodParams {
                fajr_angle: 20.0,
                isha_angle: 18.0,
                isha_minutes: None,
            },
        }
    }

    /// Human-readable display name.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::MWL => "Muslim World League (MWL)",
            Self::ISNA => "Islamic Society of North America (ISNA)",
            Self::Egyptian => "Egyptian General Authority of Survey",
            Self::Karachi => "University of Islamic Sciences, Karachi",
            Self::UmmAlQura => "Umm Al-Qura University, Makkah",
            Self::JAKIM => "Jabatan Kemajuan Islam Malaysia (JAKIM)",
        }
    }

    /// All available methods.
    pub fn all() -> &'static [CalculationMethod] {
        &[
            Self::MWL,
            Self::ISNA,
            Self::Egyptian,
            Self::Karachi,
            Self::UmmAlQura,
            Self::JAKIM,
        ]
    }

    /// Parse from a case-insensitive string. Supports common aliases.
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_lowercase().replace(['-', '_', ' '], "").as_str() {
            "mwl" | "muslimworldleague" => Some(Self::MWL),
            "isna" | "islamicsocietyofnorthamerica" => Some(Self::ISNA),
            "egyptian" | "egypt" | "egyptiangeneralauthorityofsurvey" => Some(Self::Egyptian),
            "karachi" | "universityofislamicscienceskarachi" => Some(Self::Karachi),
            "ummalqura" | "ummulqura" | "makkah" | "mecca" => Some(Self::UmmAlQura),
            "jakim" | "jabatankemajuanislammalaysia" => Some(Self::JAKIM),
            _ => None,
        }
    }
}

impl fmt::Display for CalculationMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag = match self {
            Self::MWL => "MWL",
            Self::ISNA => "ISNA",
            Self::Egyptian => "Egyptian",
            Self::Karachi => "Karachi",
            Self::UmmAlQura => "UmmAlQura",
            Self::JAKIM => "JAKIM",
        };
        write!(f, "{tag}")
    }
}
