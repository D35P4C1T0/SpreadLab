use crate::damage_bridge::{calculate_benchmark, DamageBenchmark};
use crate::data::ChampionsData;
use crate::showdown::build_champions_sp_line;
use crate::spreads::{generate_spreads, SpreadSearch};
use crate::stats::{champions_final_stats, FinalStats, StatPoints};
use damage_calc::{DamageResult, Nature};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OptimizeError {
    #[error(transparent)]
    Bridge(#[from] crate::damage_bridge::BridgeError),
    #[error(transparent)]
    Data(#[from] crate::data::DataError),
    #[error(transparent)]
    Stats(#[from] crate::stats::StatError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationMode {
    Defensive,
    Offensive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedSpread {
    pub rank: usize,
    pub mode: OptimizationMode,
    pub sps: StatPoints,
    pub sp_line: String,
    pub final_stats: FinalStats,
    pub score: f64,
    pub results: Vec<DamageSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageSummary {
    pub min_damage: u16,
    pub max_damage: u16,
    pub percent_min: f32,
    pub percent_max: f32,
    pub ko_chance: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurvivalSpread {
    pub rank: usize,
    pub nature: Nature,
    pub sps: StatPoints,
    pub sp_line: String,
    pub final_stats: FinalStats,
    pub total_points: u16,
    pub result: DamageSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedDamageSummary {
    pub min_damage: u16,
    pub max_damage: u16,
    pub percent_min: f32,
    pub percent_max: f32,
    pub ko_chance: f32,
    pub starting_hp: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedSurvivalSpread {
    pub rank: usize,
    pub nature: Nature,
    pub sps: StatPoints,
    pub sp_line: String,
    pub final_stats: FinalStats,
    pub total_points: u16,
    pub hits: Vec<DamageSummary>,
    pub combined: CombinedDamageSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurvivalSearchResult {
    pub matches: Vec<SurvivalSpread>,
    pub closest_miss: Option<SurvivalSpread>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedSurvivalSearchResult {
    pub matches: Vec<CombinedSurvivalSpread>,
    pub closest_miss: Option<CombinedSurvivalSpread>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OffensiveInvestmentStat {
    Attack,
    SpecialAttack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KoSpread {
    pub rank: usize,
    pub nature: Nature,
    pub investment_stat: OffensiveInvestmentStat,
    pub sps: StatPoints,
    pub sp_line: String,
    pub final_stats: FinalStats,
    pub total_points: u16,
    pub result: DamageSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KoSearchResult {
    pub matches: Vec<KoSpread>,
    pub closest_miss: Option<KoSpread>,
}

pub fn optimize_defensive(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
    search: SpreadSearch,
    limit: usize,
) -> Result<Vec<RankedSpread>, OptimizeError> {
    optimize(data, benchmarks, search, limit, OptimizationMode::Defensive)
}

pub fn optimize_offensive(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
    search: SpreadSearch,
    limit: usize,
) -> Result<Vec<RankedSpread>, OptimizeError> {
    optimize(data, benchmarks, search, limit, OptimizationMode::Offensive)
}

pub fn minimize_hp_def_survival(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
    natures: &[Nature],
    max_ko_chance: f32,
    limit: usize,
) -> Result<Vec<SurvivalSpread>, OptimizeError> {
    Ok(hp_def_survival_search(data, benchmark, natures, max_ko_chance, limit)?.matches)
}

pub fn hp_def_survival_search(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
    natures: &[Nature],
    max_ko_chance: f32,
    limit: usize,
) -> Result<SurvivalSearchResult, OptimizeError> {
    hp_def_survival_search_from_hp_percent(data, benchmark, natures, max_ko_chance, 100.0, limit)
}

pub fn hp_def_survival_search_from_hp_percent(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
    natures: &[Nature],
    max_ko_chance: f32,
    hp_percent: f32,
    limit: usize,
) -> Result<SurvivalSearchResult, OptimizeError> {
    let species = data.species(&benchmark.defender.species)?;
    let mut matches = Vec::new();
    let mut misses = Vec::new();

    for nature in natures {
        for hp in 0..=32 {
            for defense in 0..=32 {
                let sps = StatPoints::new(hp, 0, defense, 0, 0, 0);
                let mut candidate = benchmark.clone();
                candidate.defender.nature = *nature;
                candidate.defender.stat_points = sps;

                let result = calculate_benchmark(data, &candidate)?;
                let final_stats = champions_final_stats(species.base_stats(), *nature, sps)?;
                let damage_rolls = result.damage_rolls.clone();
                let mut summary = DamageSummary::from(result);
                summary.ko_chance = Some(ko_chance_from_rolls(
                    &damage_rolls,
                    current_hp_from_percent(final_stats.hp, hp_percent),
                ));
                let ko_chance = summary.ko_chance.unwrap_or(0.0);
                let spread = SurvivalSpread {
                    rank: 0,
                    nature: *nature,
                    sps,
                    sp_line: build_champions_sp_line(sps),
                    final_stats,
                    total_points: sps.total(),
                    result: summary,
                };
                if ko_chance <= max_ko_chance {
                    matches.push(spread);
                } else {
                    misses.push(spread);
                }
            }
        }
    }

    matches.sort_by(|left, right| {
        left.total_points
            .cmp(&right.total_points)
            .then_with(|| {
                left.result
                    .ko_chance
                    .partial_cmp(&right.result.ko_chance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.result.max_damage.cmp(&right.result.max_damage))
            .then_with(|| left.sps.hp.cmp(&right.sps.hp))
            .then_with(|| left.sps.defense.cmp(&right.sps.defense))
    });
    matches.truncate(limit);
    for (index, spread) in matches.iter_mut().enumerate() {
        spread.rank = index + 1;
    }

    misses.sort_by(|left, right| {
        left.result
            .ko_chance
            .partial_cmp(&right.result.ko_chance)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                left.result
                    .percent_max
                    .partial_cmp(&right.result.percent_max)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.result.max_damage.cmp(&right.result.max_damage))
            .then_with(|| left.total_points.cmp(&right.total_points))
    });
    let mut closest_miss = misses.into_iter().next();
    if let Some(spread) = &mut closest_miss {
        spread.rank = 1;
    }

    Ok(SurvivalSearchResult {
        matches,
        closest_miss,
    })
}

pub fn hp_def_combined_survival_search(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
    natures: &[Nature],
    max_ko_chance: f32,
    hp_percent: f32,
    limit: usize,
) -> Result<CombinedSurvivalSearchResult, OptimizeError> {
    let Some(first_benchmark) = benchmarks.first() else {
        return Ok(CombinedSurvivalSearchResult {
            matches: Vec::new(),
            closest_miss: None,
        });
    };
    let species = data.species(&first_benchmark.defender.species)?;
    let mut matches = Vec::new();
    let mut misses = Vec::new();

    for nature in natures {
        for hp in 0..=32 {
            for defense in 0..=32 {
                let sps = StatPoints::new(hp, 0, defense, 0, 0, 0);
                let final_stats = champions_final_stats(species.base_stats(), *nature, sps)?;
                let starting_hp = current_hp_from_percent(final_stats.hp, hp_percent);
                let mut hits = Vec::with_capacity(benchmarks.len());

                for benchmark in benchmarks {
                    let mut candidate = benchmark.clone();
                    candidate.defender.nature = *nature;
                    candidate.defender.stat_points = sps;
                    candidate.defender_current_hp = Some(starting_hp);
                    let result = calculate_benchmark(data, &candidate)?;
                    hits.push(DamageSummary::from(result));
                }

                let combined = sequence_damage_summary(
                    data,
                    benchmarks,
                    *nature,
                    sps,
                    final_stats.hp,
                    starting_hp,
                )?;
                let spread = CombinedSurvivalSpread {
                    rank: 0,
                    nature: *nature,
                    sps,
                    sp_line: build_champions_sp_line(sps),
                    final_stats,
                    total_points: sps.total(),
                    hits,
                    combined,
                };
                if spread.combined.ko_chance <= max_ko_chance {
                    matches.push(spread);
                } else {
                    misses.push(spread);
                }
            }
        }
    }

    matches.sort_by(|left, right| {
        left.total_points
            .cmp(&right.total_points)
            .then_with(|| {
                left.combined
                    .ko_chance
                    .partial_cmp(&right.combined.ko_chance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.combined.max_damage.cmp(&right.combined.max_damage))
            .then_with(|| left.sps.hp.cmp(&right.sps.hp))
            .then_with(|| left.sps.defense.cmp(&right.sps.defense))
    });
    matches.truncate(limit);
    for (index, spread) in matches.iter_mut().enumerate() {
        spread.rank = index + 1;
    }

    misses.sort_by(|left, right| {
        left.combined
            .ko_chance
            .partial_cmp(&right.combined.ko_chance)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                left.combined
                    .percent_max
                    .partial_cmp(&right.combined.percent_max)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.combined.max_damage.cmp(&right.combined.max_damage))
            .then_with(|| left.total_points.cmp(&right.total_points))
    });
    let mut closest_miss = misses.into_iter().next();
    if let Some(spread) = &mut closest_miss {
        spread.rank = 1;
    }

    Ok(CombinedSurvivalSearchResult {
        matches,
        closest_miss,
    })
}

pub fn minimize_offensive_ko(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
    natures: &[Nature],
    min_ko_chance: f32,
    limit: usize,
) -> Result<Vec<KoSpread>, OptimizeError> {
    Ok(offensive_ko_search(data, benchmark, natures, min_ko_chance, limit)?.matches)
}

pub fn offensive_ko_search(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
    natures: &[Nature],
    min_ko_chance: f32,
    limit: usize,
) -> Result<KoSearchResult, OptimizeError> {
    let species = data.species(&benchmark.attacker.species)?;
    let move_data = data.move_data(&benchmark.move_name)?;
    let investment_stat = match move_data.category.as_str() {
        "Physical" => OffensiveInvestmentStat::Attack,
        "Special" => OffensiveInvestmentStat::SpecialAttack,
        _ => OffensiveInvestmentStat::Attack,
    };
    let mut matches = Vec::new();
    let mut misses = Vec::new();

    for nature in natures {
        for points in 0..=32 {
            let sps = match investment_stat {
                OffensiveInvestmentStat::Attack => StatPoints::new(0, points, 0, 0, 0, 0),
                OffensiveInvestmentStat::SpecialAttack => StatPoints::new(0, 0, 0, points, 0, 0),
            };
            let mut candidate = benchmark.clone();
            candidate.attacker.nature = *nature;
            candidate.attacker.stat_points = sps;

            let result = calculate_benchmark(data, &candidate)?;
            let ko_chance = result.ko_chance.unwrap_or(0.0);
            let spread = KoSpread {
                rank: 0,
                nature: *nature,
                investment_stat,
                sps,
                sp_line: build_champions_sp_line(sps),
                final_stats: champions_final_stats(species.base_stats(), *nature, sps)?,
                total_points: sps.total(),
                result: DamageSummary::from(result),
            };
            if ko_chance >= min_ko_chance {
                matches.push(spread);
            } else {
                misses.push(spread);
            }
        }
    }

    matches.sort_by(|left, right| {
        left.total_points
            .cmp(&right.total_points)
            .then_with(|| {
                right
                    .result
                    .ko_chance
                    .partial_cmp(&left.result.ko_chance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| right.result.min_damage.cmp(&left.result.min_damage))
    });
    matches.truncate(limit);
    for (index, spread) in matches.iter_mut().enumerate() {
        spread.rank = index + 1;
    }

    misses.sort_by(|left, right| {
        right
            .result
            .ko_chance
            .partial_cmp(&left.result.ko_chance)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                right
                    .result
                    .percent_min
                    .partial_cmp(&left.result.percent_min)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| right.result.min_damage.cmp(&left.result.min_damage))
            .then_with(|| left.total_points.cmp(&right.total_points))
    });
    let mut closest_miss = misses.into_iter().next();
    if let Some(spread) = &mut closest_miss {
        spread.rank = 1;
    }

    Ok(KoSearchResult {
        matches,
        closest_miss,
    })
}

pub fn all_natures() -> [Nature; 25] {
    [
        Nature::Adamant,
        Nature::Bashful,
        Nature::Bold,
        Nature::Brave,
        Nature::Calm,
        Nature::Careful,
        Nature::Docile,
        Nature::Gentle,
        Nature::Hardy,
        Nature::Hasty,
        Nature::Impish,
        Nature::Jolly,
        Nature::Lax,
        Nature::Lonely,
        Nature::Mild,
        Nature::Modest,
        Nature::Naive,
        Nature::Naughty,
        Nature::Quiet,
        Nature::Quirky,
        Nature::Rash,
        Nature::Relaxed,
        Nature::Sassy,
        Nature::Serious,
        Nature::Timid,
    ]
}

pub fn optimized_offensive_natures(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
) -> Result<[Nature; 2], OptimizeError> {
    let move_data = data.move_data(&benchmark.move_name)?;
    Ok(match move_data.category.as_str() {
        "Special" => [Nature::Modest, Nature::Hardy],
        _ => [Nature::Adamant, Nature::Hardy],
    })
}

pub fn optimized_defensive_natures(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
) -> Result<[Nature; 2], OptimizeError> {
    let move_data = data.move_data(&benchmark.move_name)?;
    Ok(match move_data.category.as_str() {
        "Special" => [Nature::Calm, Nature::Hardy],
        _ => [Nature::Bold, Nature::Hardy],
    })
}

pub fn optimized_combined_defensive_natures(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
) -> Result<Vec<Nature>, OptimizeError> {
    let mut has_physical = false;
    let mut has_special = false;
    for benchmark in benchmarks {
        let move_data = data.move_data(&benchmark.move_name)?;
        match move_data.category.as_str() {
            "Special" => has_special = true,
            "Physical" => has_physical = true,
            _ => {}
        }
    }
    Ok(match (has_physical, has_special) {
        (true, true) => vec![Nature::Bold, Nature::Calm, Nature::Hardy],
        (true, false) => vec![Nature::Bold, Nature::Hardy],
        (false, true) => vec![Nature::Calm, Nature::Hardy],
        (false, false) => vec![Nature::Hardy],
    })
}

fn optimize(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
    search: SpreadSearch,
    limit: usize,
    mode: OptimizationMode,
) -> Result<Vec<RankedSpread>, OptimizeError> {
    let mut ranked = Vec::new();

    for sps in generate_spreads(search) {
        let mut summaries = Vec::with_capacity(benchmarks.len());
        let mut score = 0.0;
        let final_stats = match mode {
            OptimizationMode::Defensive => {
                let Some(first) = benchmarks.first() else {
                    continue;
                };
                let species = data.species(&first.defender.species)?;
                champions_final_stats(species.base_stats(), first.defender.nature, sps)?
            }
            OptimizationMode::Offensive => {
                let Some(first) = benchmarks.first() else {
                    continue;
                };
                let species = data.species(&first.attacker.species)?;
                champions_final_stats(species.base_stats(), first.attacker.nature, sps)?
            }
        };

        for benchmark in benchmarks {
            let mut candidate = benchmark.clone();
            match mode {
                OptimizationMode::Defensive => {
                    candidate.defender.stat_points = sps;
                }
                OptimizationMode::Offensive => {
                    candidate.attacker.stat_points = sps;
                }
            }

            let result = calculate_benchmark(data, &candidate)?;
            score += score_result(mode, &result);
            summaries.push(DamageSummary::from(result));
        }

        ranked.push(RankedSpread {
            rank: 0,
            mode,
            sps,
            sp_line: build_champions_sp_line(sps),
            final_stats,
            score,
            results: summaries,
        });
    }

    ranked.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.sps.total().cmp(&right.sps.total()))
    });
    ranked.truncate(limit);
    for (index, spread) in ranked.iter_mut().enumerate() {
        spread.rank = index + 1;
    }
    Ok(ranked)
}

fn score_result(mode: OptimizationMode, result: &DamageResult) -> f64 {
    let ko = result.ko_chance.unwrap_or(0.0) as f64;
    match mode {
        OptimizationMode::Defensive => {
            let max_percent = result.percent_range.1 as f64;
            (1.0 - ko) * 10_000.0 - max_percent
        }
        OptimizationMode::Offensive => {
            let min_percent = result.percent_range.0 as f64;
            ko * 10_000.0 + min_percent
        }
    }
}

impl From<DamageResult> for DamageSummary {
    fn from(value: DamageResult) -> Self {
        Self {
            min_damage: value.min_damage,
            max_damage: value.max_damage,
            percent_min: value.percent_range.0,
            percent_max: value.percent_range.1,
            ko_chance: value.ko_chance,
        }
    }
}

fn current_hp_from_percent(max_hp: u16, hp_percent: f32) -> u16 {
    let percent = hp_percent.clamp(0.0, 100.0);
    let hp = ((max_hp as f32) * percent / 100.0).ceil() as u16;
    hp.clamp(1, max_hp)
}

fn ko_chance_from_rolls(rolls: &[u16], starting_hp: u16) -> f32 {
    if rolls.is_empty() {
        return 0.0;
    }
    let ko_rolls = rolls
        .iter()
        .filter(|damage| **damage >= starting_hp)
        .count();
    ko_rolls as f32 / rolls.len() as f32
}

fn sequence_damage_summary(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
    nature: Nature,
    sps: StatPoints,
    max_hp: u16,
    starting_hp: u16,
) -> Result<CombinedDamageSummary, OptimizeError> {
    let mut total_probability = 0.0;
    let mut ko_probability = 0.0;
    let mut min_damage = u16::MAX;
    let mut max_damage = 0u16;
    let mut cache = HashMap::new();
    count_sequence_rolls(
        data,
        benchmarks,
        nature,
        sps,
        &mut cache,
        0,
        starting_hp,
        0,
        1.0,
        &mut total_probability,
        &mut ko_probability,
        &mut min_damage,
        &mut max_damage,
    )?;
    if min_damage == u16::MAX {
        min_damage = 0;
    }
    Ok(CombinedDamageSummary {
        min_damage,
        max_damage,
        percent_min: min_damage as f32 * 100.0 / max_hp as f32,
        percent_max: max_damage as f32 * 100.0 / max_hp as f32,
        ko_chance: if total_probability == 0.0 {
            0.0
        } else {
            (ko_probability / total_probability) as f32
        },
        starting_hp,
    })
}

#[allow(clippy::too_many_arguments)]
fn count_sequence_rolls(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
    nature: Nature,
    sps: StatPoints,
    cache: &mut HashMap<(usize, u16), Vec<u16>>,
    index: usize,
    current_hp: u16,
    running_damage: u16,
    probability: f64,
    total_probability: &mut f64,
    ko_probability: &mut f64,
    min_damage: &mut u16,
    max_damage: &mut u16,
) -> Result<(), OptimizeError> {
    if index == benchmarks.len() {
        *total_probability += probability;
        *min_damage = (*min_damage).min(running_damage);
        *max_damage = (*max_damage).max(running_damage);
        return Ok(());
    }

    let rolls = if let Some(rolls) = cache.get(&(index, current_hp)).cloned() {
        rolls
    } else {
        let mut candidate = benchmarks[index].clone();
        candidate.defender.nature = nature;
        candidate.defender.stat_points = sps;
        candidate.defender_current_hp = Some(current_hp);
        let result = calculate_benchmark(data, &candidate)?;
        cache.insert((index, current_hp), result.damage_rolls.clone());
        result.damage_rolls
    };

    if rolls.is_empty() {
        *total_probability += probability;
        *min_damage = (*min_damage).min(running_damage);
        *max_damage = (*max_damage).max(running_damage);
        return Ok(());
    }

    let roll_probability = probability / rolls.len() as f64;
    for damage in rolls {
        let next_damage = running_damage.saturating_add(damage);
        if damage >= current_hp {
            *total_probability += roll_probability;
            *ko_probability += roll_probability;
            *min_damage = (*min_damage).min(next_damage);
            *max_damage = (*max_damage).max(next_damage);
            continue;
        }
        count_sequence_rolls(
            data,
            benchmarks,
            nature,
            sps,
            cache,
            index + 1,
            current_hp - damage,
            next_damage,
            roll_probability,
            total_probability,
            ko_probability,
            min_damage,
            max_damage,
        )?;
    }
    Ok(())
}
