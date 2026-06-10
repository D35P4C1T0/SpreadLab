use crate::data::{parse_spread_key, DataError};
use crate::stats::StatPoints;
use directories::ProjectDirs;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const DEFAULT_FORMAT: &str = "gen9championsvgc2026regmabo3";
pub const DEFAULT_DISPLAY_NAME: &str = "Champions VGC 2026 Reg M-A (Bo3)";
pub const DEFAULT_RATING: u16 = 1760;
pub const SUPPORTED_RATINGS: &[u16] = &[0, 1500, 1630, 1760];

#[derive(Debug, Error)]
pub enum SmogonError {
    #[error("unsupported rating {0}; supported ratings: 0, 1500, 1630, 1760")]
    UnsupportedRating(u16),
    #[error("could not find chaos data for {format} rating {rating}")]
    LatestNotFound { format: String, rating: u16 },
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("io failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("json parse failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Data(#[from] DataError),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetagameStats {
    pub format_id: String,
    pub display_name: String,
    pub month: String,
    pub rating: u16,
    pub battles: u64,
    pub pokemon: Vec<PokemonUsage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PokemonUsage {
    pub name: String,
    pub usage: f64,
    pub raw_count: u64,
    pub abilities: Vec<WeightedName>,
    pub items: Vec<WeightedName>,
    pub moves: Vec<WeightedName>,
    pub spreads: Vec<WeightedSpread>,
    pub tera_types: Vec<WeightedName>,
    pub teammates: Vec<WeightedName>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeightedName {
    pub name: String,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeightedSpread {
    pub nature: damage_calc::Nature,
    pub sps: StatPoints,
    pub weight: f64,
}

#[derive(Debug, Deserialize)]
struct LegacyChaos {
    info: LegacyInfo,
    data: HashMap<String, LegacyPokemon>,
}

#[derive(Debug, Deserialize)]
struct LegacyInfo {
    metagame: String,
    cutoff: u16,
    #[serde(rename = "number of battles")]
    number_of_battles: u64,
}

#[derive(Debug, Deserialize)]
struct LegacyPokemon {
    #[serde(rename = "Raw count")]
    raw_count: u64,
    usage: f64,
    #[serde(rename = "Abilities", default)]
    abilities: HashMap<String, f64>,
    #[serde(rename = "Items", default)]
    items: HashMap<String, f64>,
    #[serde(rename = "Moves", default)]
    moves: HashMap<String, f64>,
    #[serde(rename = "Spreads", default)]
    spreads: HashMap<String, f64>,
    #[serde(rename = "Tera Types", default)]
    tera_types: HashMap<String, f64>,
    #[serde(rename = "Teammates", default)]
    teammates: HashMap<String, f64>,
}

pub fn fetch_month(
    month: &str,
    format: &str,
    rating: u16,
    cache_dir: Option<&Path>,
) -> Result<MetagameStats, SmogonError> {
    validate_rating(rating)?;
    let bytes = download_month(month, format, rating)?;
    let stats = normalize_chaos(month, DEFAULT_DISPLAY_NAME, &bytes)?;
    if let Some(cache_dir) = cache_dir {
        fs::create_dir_all(cache_dir.join(month))?;
        fs::write(cache_path(cache_dir, month, format, rating), &bytes)?;
    }
    Ok(stats)
}

pub fn fetch_latest(
    format: &str,
    rating: u16,
    cache_dir: Option<&Path>,
) -> Result<MetagameStats, SmogonError> {
    validate_rating(rating)?;
    for month in candidate_months(2026, 6, 24) {
        match fetch_month(&month, format, rating, cache_dir) {
            Ok(stats) => return Ok(stats),
            Err(SmogonError::Request(error))
                if error.status() == Some(reqwest::StatusCode::NOT_FOUND) => {}
            Err(error) => return Err(error),
        }
    }
    Err(SmogonError::LatestNotFound {
        format: format.to_owned(),
        rating,
    })
}

pub fn normalize_chaos(
    month: &str,
    display_name: &str,
    bytes: &[u8],
) -> Result<MetagameStats, SmogonError> {
    let chaos: LegacyChaos = serde_json::from_slice(bytes)?;
    let mut pokemon = chaos
        .data
        .into_iter()
        .map(|(name, entry)| {
            let spreads = entry
                .spreads
                .into_iter()
                .filter_map(|(raw, weight)| {
                    parse_spread_key(&raw)
                        .ok()
                        .map(|(nature, sps)| WeightedSpread {
                            nature,
                            sps,
                            weight,
                        })
                })
                .collect::<Vec<_>>();
            PokemonUsage {
                name,
                usage: entry.usage,
                raw_count: entry.raw_count,
                abilities: weighted(entry.abilities),
                items: weighted(entry.items),
                moves: weighted(entry.moves),
                spreads,
                tera_types: weighted(entry.tera_types),
                teammates: weighted(entry.teammates),
            }
        })
        .collect::<Vec<_>>();
    pokemon.sort_by(|left, right| {
        right
            .usage
            .partial_cmp(&left.usage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(MetagameStats {
        format_id: chaos.info.metagame,
        display_name: display_name.to_owned(),
        month: month.to_owned(),
        rating: chaos.info.cutoff,
        battles: chaos.info.number_of_battles,
        pokemon,
    })
}

pub fn default_cache_dir() -> Option<PathBuf> {
    ProjectDirs::from("dev", "D35P4C1T0", "SpreadLab").map(|dirs| dirs.cache_dir().join("smogon"))
}

fn download_month(month: &str, format: &str, rating: u16) -> Result<Vec<u8>, SmogonError> {
    let gz_url = format!("https://www.smogon.com/stats/{month}/chaos/{format}-{rating}.json.gz");
    let response = reqwest::blocking::get(&gz_url)?;
    if response.status().is_success() {
        let compressed = response.bytes()?;
        let mut decoder = GzDecoder::new(compressed.as_ref());
        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded)?;
        return Ok(decoded);
    }
    if response.status() != reqwest::StatusCode::NOT_FOUND {
        response.error_for_status()?;
    }

    let json_url = format!("https://www.smogon.com/stats/{month}/chaos/{format}-{rating}.json");
    let response = reqwest::blocking::get(json_url)?;
    response.error_for_status_ref()?;
    Ok(response.bytes()?.to_vec())
}

fn weighted(map: HashMap<String, f64>) -> Vec<WeightedName> {
    let mut values = map
        .into_iter()
        .map(|(name, weight)| WeightedName { name, weight })
        .collect::<Vec<_>>();
    values.sort_by(|left, right| {
        right
            .weight
            .partial_cmp(&left.weight)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    values
}

fn validate_rating(rating: u16) -> Result<(), SmogonError> {
    if SUPPORTED_RATINGS.contains(&rating) {
        Ok(())
    } else {
        Err(SmogonError::UnsupportedRating(rating))
    }
}

fn cache_path(cache_dir: &Path, month: &str, format: &str, rating: u16) -> PathBuf {
    cache_dir
        .join(month)
        .join(format!("{format}-{rating}.json"))
}

fn candidate_months(mut year: i32, mut month: u8, count: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(format!("{year:04}-{month:02}"));
        if month == 1 {
            year -= 1;
            month = 12;
        } else {
            month -= 1;
        }
    }
    out
}
