use crate::stats::{StatError, StatPoints, MAX_STAT_POINTS, MAX_TOTAL_STAT_POINTS};
use damage_calc::Nature;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ShowdownError {
    #[error("Showdown set is malformed: include only one training line type, either EVs or SPs")]
    MixedTrainingLines,
    #[error("Showdown set is malformed: include only one EVs/SPs line")]
    MultipleTrainingLines,
    #[error("Showdown set has a malformed {0} line")]
    MalformedTrainingLine(&'static str),
    #[error("Showdown set SPs exceed the {MAX_TOTAL_STAT_POINTS}-point cap")]
    StatPointsOverCap,
    #[error("unknown nature: {0}")]
    UnknownNature(String),
    #[error(transparent)]
    Stat(#[from] StatError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingFormat {
    Evs,
    Sps,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedSet {
    pub species: String,
    pub item: Option<String>,
    pub ability: Option<String>,
    pub nature: Nature,
    pub tera_type: Option<String>,
    pub moves: Vec<String>,
    pub training_format: Option<TrainingFormat>,
    pub stat_points: StatPoints,
    pub original_text: String,
}

#[derive(Debug, Clone, Copy)]
struct TrainingLine<'a> {
    format: TrainingFormat,
    payload: &'a str,
}

pub fn champions_points_from_ev(value: u16) -> u16 {
    MAX_STAT_POINTS.min((value + 4) / 8)
}

pub fn approximate_ev_from_champions(value: u16) -> u16 {
    let points = MAX_STAT_POINTS.min(value);
    if points == 0 {
        0
    } else {
        252.min(4 + (points - 1) * 8)
    }
}

pub fn parse_set(text: &str) -> Result<ParsedSet, ShowdownError> {
    let normalized = normalize_text(text);
    let species = parse_species(&normalized).unwrap_or_default();
    let item = parse_item(&normalized);
    let ability = parse_prefixed_line(&normalized, "Ability:");
    let nature = parse_nature(&normalized)?.unwrap_or(Nature::Hardy);
    let tera_type = parse_prefixed_line(&normalized, "Tera Type:");
    let moves = normalized
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- "))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    let (training_format, stat_points) = parse_training(&normalized)?;

    Ok(ParsedSet {
        species,
        item,
        ability,
        nature,
        tera_type,
        moves,
        training_format,
        stat_points,
        original_text: normalized,
    })
}

pub fn build_champions_sp_line(points: StatPoints) -> String {
    build_training_line("SPs", points, |value| value)
}

pub fn build_approximate_legacy_ev_line(points: StatPoints) -> String {
    build_training_line("EVs", points, approximate_ev_from_champions)
}

fn parse_training(text: &str) -> Result<(Option<TrainingFormat>, StatPoints), ShowdownError> {
    let lines = collect_training_lines(text);
    if lines.is_empty() {
        return Ok((None, StatPoints::default()));
    }
    if lines.len() > 1 {
        let has_evs = lines.iter().any(|line| line.format == TrainingFormat::Evs);
        let has_sps = lines.iter().any(|line| line.format == TrainingFormat::Sps);
        return if has_evs && has_sps {
            Err(ShowdownError::MixedTrainingLines)
        } else {
            Err(ShowdownError::MultipleTrainingLines)
        };
    }

    let line = lines[0];
    let points = match line.format {
        TrainingFormat::Evs => parse_evs_payload(line.payload)?,
        TrainingFormat::Sps => parse_sps_payload(line.payload)?,
    };
    points.validate()?;

    Ok((Some(line.format), points))
}

fn parse_evs_payload(payload: &str) -> Result<StatPoints, ShowdownError> {
    parse_training_payload(payload, 252, champions_points_from_ev, "EVs")
}

fn parse_sps_payload(payload: &str) -> Result<StatPoints, ShowdownError> {
    parse_training_payload(payload, MAX_STAT_POINTS, |value| value, "SPs")
}

fn parse_training_payload(
    payload: &str,
    max_value: u16,
    convert: impl Fn(u16) -> u16,
    label: &'static str,
) -> Result<StatPoints, ShowdownError> {
    let mut points = StatPoints::default();
    let mut parsed = 0;

    for segment in payload.split('/') {
        let mut parts = segment.split_whitespace();
        let Some(raw_value) = parts.next() else {
            continue;
        };
        let Some(raw_stat) = parts.next() else {
            continue;
        };
        if parts.next().is_some() {
            continue;
        }
        let Ok(value) = raw_value.parse::<u16>() else {
            continue;
        };
        let value = max_value.min(value);
        let value = convert(value);
        match raw_stat.to_ascii_lowercase().as_str() {
            "hp" => points.hp = value,
            "atk" => points.attack = value,
            "def" => points.defense = value,
            "spa" => points.special_attack = value,
            "spd" => points.special_defense = value,
            "spe" => points.speed = value,
            _ => continue,
        }
        parsed += 1;
    }

    if parsed == 0 {
        return Err(ShowdownError::MalformedTrainingLine(label));
    }
    if points.total() > MAX_TOTAL_STAT_POINTS {
        return Err(ShowdownError::StatPointsOverCap);
    }
    Ok(points)
}

fn collect_training_lines(text: &str) -> Vec<TrainingLine<'_>> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let (label, payload) = trimmed.split_once(':')?;
            let format = match label.to_ascii_lowercase().as_str() {
                "evs" => TrainingFormat::Evs,
                "sps" => TrainingFormat::Sps,
                _ => return None,
            };
            Some(TrainingLine {
                format,
                payload: payload.trim(),
            })
        })
        .collect()
}

fn parse_species(text: &str) -> Option<String> {
    let first = text.lines().find(|line| !line.trim().is_empty())?.trim();
    let before_item = first.split_once('@').map_or(first, |(left, _)| left).trim();
    (!before_item.is_empty()).then(|| before_item.to_owned())
}

fn parse_item(text: &str) -> Option<String> {
    let first = text.lines().find(|line| !line.trim().is_empty())?.trim();
    first
        .split_once('@')
        .map(|(_, item)| item.trim().to_owned())
        .filter(|item| !item.is_empty())
}

fn parse_prefixed_line(text: &str, prefix: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix(prefix))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_nature(text: &str) -> Result<Option<Nature>, ShowdownError> {
    let Some(line) = text
        .lines()
        .map(str::trim)
        .find(|line| line.ends_with(" Nature"))
    else {
        return Ok(None);
    };
    let raw = line.trim_end_matches(" Nature").trim();
    parse_nature_name(raw)
        .map(Some)
        .ok_or_else(|| ShowdownError::UnknownNature(raw.to_owned()))
}

pub fn parse_nature_name(raw: &str) -> Option<Nature> {
    Some(match raw.to_ascii_lowercase().as_str() {
        "adamant" => Nature::Adamant,
        "bashful" => Nature::Bashful,
        "bold" => Nature::Bold,
        "brave" => Nature::Brave,
        "calm" => Nature::Calm,
        "careful" => Nature::Careful,
        "docile" => Nature::Docile,
        "gentle" => Nature::Gentle,
        "hardy" => Nature::Hardy,
        "hasty" => Nature::Hasty,
        "impish" => Nature::Impish,
        "jolly" => Nature::Jolly,
        "lax" => Nature::Lax,
        "lonely" => Nature::Lonely,
        "mild" => Nature::Mild,
        "modest" => Nature::Modest,
        "naive" => Nature::Naive,
        "naughty" => Nature::Naughty,
        "quiet" => Nature::Quiet,
        "quirky" => Nature::Quirky,
        "rash" => Nature::Rash,
        "relaxed" => Nature::Relaxed,
        "sassy" => Nature::Sassy,
        "serious" => Nature::Serious,
        "timid" => Nature::Timid,
        _ => return None,
    })
}

fn build_training_line(label: &str, points: StatPoints, map: impl Fn(u16) -> u16) -> String {
    let parts = [
        ("HP", points.hp),
        ("Atk", points.attack),
        ("Def", points.defense),
        ("SpA", points.special_attack),
        ("SpD", points.special_defense),
        ("Spe", points.speed),
    ]
    .into_iter()
    .filter(|(_, value)| *value > 0)
    .map(|(stat, value)| format!("{} {}", map(value), stat))
    .collect::<Vec<_>>();

    format!(
        "{label}: {}",
        if parts.is_empty() {
            "0 HP".to_owned()
        } else {
            parts.join(" / ")
        }
    )
}

fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .filter(|ch| *ch == '\n' || *ch == '\t' || !ch.is_control())
        .collect::<String>()
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_evs_to_sps() {
        assert_eq!(champions_points_from_ev(0), 0);
        assert_eq!(champions_points_from_ev(4), 1);
        assert_eq!(champions_points_from_ev(12), 2);
        assert_eq!(champions_points_from_ev(252), 32);
    }

    #[test]
    fn parses_evs_line() {
        let parsed = parse_set("Pikachu @ Light Ball\nAbility: Static\nEVs: 252 Atk / 4 SpD / 252 Spe\nJolly Nature\n- Volt Tackle").unwrap();
        assert_eq!(parsed.species, "Pikachu");
        assert_eq!(parsed.item.as_deref(), Some("Light Ball"));
        assert_eq!(parsed.ability.as_deref(), Some("Static"));
        assert_eq!(parsed.nature, Nature::Jolly);
        assert_eq!(parsed.stat_points, StatPoints::new(0, 32, 0, 0, 1, 32));
    }

    #[test]
    fn rejects_mixed_training_lines() {
        let err = parse_set("Pikachu\nEVs: 252 Atk\nSPs: 32 Atk\nJolly Nature").unwrap_err();
        assert_eq!(err, ShowdownError::MixedTrainingLines);
    }

    #[test]
    fn exports_training_lines() {
        let points = StatPoints::new(0, 32, 0, 0, 1, 32);
        assert_eq!(
            build_champions_sp_line(points),
            "SPs: 32 Atk / 1 SpD / 32 Spe"
        );
        assert_eq!(
            build_approximate_legacy_ev_line(points),
            "EVs: 252 Atk / 4 SpD / 252 Spe"
        );
    }
}
