use crate::damage_bridge::{calculate_benchmark, DamageBenchmark};
use crate::data::{ChampionsData, DataError};
use crate::optimize::{
    all_natures, hp_def_combined_survival_search, hp_def_survival_search_from_hp_percent,
    offensive_ko_search, optimize_defensive, optimize_offensive,
    optimized_combined_defensive_natures, optimized_defensive_natures, optimized_offensive_natures,
    CombinedSurvivalSpread, DamageSummary, KoSpread, OptimizeError, RankedSpread, SurvivalSpread,
};
use crate::showdown::{parse_set, ShowdownError};
use crate::spreads::{LockedStats, SpreadSearch};
use damage_calc::{DamageResult, Field, Format, Nature, SideConditions, Terrain, Weather};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error(transparent)]
    Data(#[from] DataError),
    #[error(transparent)]
    Showdown(#[from] ShowdownError),
    #[error(transparent)]
    Optimize(#[from] OptimizeError),
    #[error(transparent)]
    Bridge(#[from] crate::damage_bridge::BridgeError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageRequest {
    pub attacker_set: String,
    pub defender_set: String,
    pub move_name: String,
    pub move_times_affected: u8,
    #[serde(default)]
    pub field: Option<FieldRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageResponse {
    pub summary: DamageSummary,
    pub rolls: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataResponse {
    pub species: Vec<String>,
    pub regulation: Vec<String>,
    pub items: Vec<String>,
    pub abilities: Vec<String>,
    pub moves: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizeBenchmarkRequest {
    pub attacker_set: String,
    pub defender_set: String,
    pub move_name: String,
    pub move_times_affected: u8,
    #[serde(default)]
    pub field: Option<FieldRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizeRequest {
    pub benchmarks: Vec<OptimizeBenchmarkRequest>,
    pub full_spend: bool,
    pub locked: LockedStatsRequest,
    pub limit: usize,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct LockedStatsRequest {
    pub hp: Option<u16>,
    pub attack: Option<u16>,
    pub defense: Option<u16>,
    pub special_attack: Option<u16>,
    pub special_defense: Option<u16>,
    pub speed: Option<u16>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct FieldRequest {
    pub format: Option<Format>,
    pub weather: Option<Weather>,
    pub terrain: Option<Terrain>,
    #[serde(default)]
    pub gravity: bool,
    #[serde(default)]
    pub fairy_aura: bool,
    #[serde(default)]
    pub protect: bool,
    #[serde(default)]
    pub helping_hand: bool,
    #[serde(default)]
    pub attacker_tailwind: bool,
    #[serde(default)]
    pub defender_tailwind: bool,
    #[serde(default)]
    pub defender_reflect: bool,
    #[serde(default)]
    pub defender_light_screen: bool,
    #[serde(default)]
    pub defender_aurora_veil: bool,
    #[serde(default)]
    pub defender_friend_guard: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HpDefSurvivalRequest {
    pub attacker_set: String,
    pub defender_set: String,
    pub move_name: String,
    pub max_ko_chance: f32,
    pub hp_percent: Option<f32>,
    pub nature: Option<Nature>,
    pub optimize_nature: bool,
    pub limit: usize,
    pub move_times_affected: u8,
    #[serde(default)]
    pub field: Option<FieldRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingHitRequest {
    pub attacker_set: String,
    pub move_name: String,
    pub move_times_affected: u8,
    #[serde(default)]
    pub field: Option<FieldRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedHpDefSurvivalRequest {
    pub defender_set: String,
    pub hits: Vec<IncomingHitRequest>,
    pub max_ko_chance: f32,
    pub hp_percent: Option<f32>,
    pub nature: Option<Nature>,
    pub optimize_nature: bool,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffensiveKoRequest {
    pub attacker_set: String,
    pub defender_set: String,
    pub move_name: String,
    pub min_ko_chance: f32,
    pub nature: Option<Nature>,
    pub optimize_nature: bool,
    pub limit: usize,
    pub move_times_affected: u8,
    #[serde(default)]
    pub field: Option<FieldRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffensiveKoResponse {
    pub best: Option<KoSpread>,
    pub matches: Vec<KoSpread>,
    pub closest_miss: Option<KoSpread>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HpDefSurvivalResponse {
    pub best: Option<SurvivalSpread>,
    pub matches: Vec<SurvivalSpread>,
    pub closest_miss: Option<SurvivalSpread>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedHpDefSurvivalResponse {
    pub best: Option<CombinedSurvivalSpread>,
    pub matches: Vec<CombinedSurvivalSpread>,
    pub closest_miss: Option<CombinedSurvivalSpread>,
}

impl HpDefSurvivalRequest {
    pub fn with_default_limit(mut self) -> Self {
        if self.limit == 0 {
            self.limit = 10;
        }
        self
    }
}

pub fn load_metadata() -> Result<MetadataResponse, ApiError> {
    let data = ChampionsData::load()?;
    let mut response = MetadataResponse {
        species: data.species_names().map(str::to_owned).collect(),
        regulation: data.regulation_m_a_names().map(str::to_owned).collect(),
        items: data.item_names().map(str::to_owned).collect(),
        abilities: data.ability_names().map(str::to_owned).collect(),
        moves: data.move_names().map(str::to_owned).collect(),
    };
    sort_unique(&mut response.species);
    sort_unique(&mut response.regulation);
    sort_unique(&mut response.items);
    sort_unique(&mut response.abilities);
    sort_unique(&mut response.moves);
    Ok(response)
}

pub fn calculate_damage_request(request: DamageRequest) -> Result<DamageResponse, ApiError> {
    let data = ChampionsData::load()?;
    calculate_damage_request_with_data(&data, request)
}

pub fn calculate_damage_request_with_data(
    data: &ChampionsData,
    request: DamageRequest,
) -> Result<DamageResponse, ApiError> {
    let benchmark = benchmark_from_sets(
        &request.attacker_set,
        &request.defender_set,
        request.move_name,
        request.move_times_affected,
        request.field,
    )?;
    let result = calculate_benchmark(data, &benchmark)?;
    Ok(DamageResponse::from(result))
}

pub fn find_min_hp_def_survival(
    request: HpDefSurvivalRequest,
) -> Result<HpDefSurvivalResponse, ApiError> {
    let data = ChampionsData::load()?;
    find_min_hp_def_survival_with_data(&data, request)
}

pub fn find_min_hp_def_survival_with_data(
    data: &ChampionsData,
    request: HpDefSurvivalRequest,
) -> Result<HpDefSurvivalResponse, ApiError> {
    let request = request.with_default_limit();
    let benchmark = benchmark_from_sets(
        &request.attacker_set,
        &request.defender_set,
        request.move_name,
        request.move_times_affected,
        request.field,
    )?;
    let owned_natures;
    let natures = if let Some(nature) = request.nature {
        owned_natures = vec![nature];
        owned_natures.as_slice()
    } else if request.optimize_nature {
        owned_natures = optimized_defensive_natures(data, &benchmark)?.to_vec();
        owned_natures.as_slice()
    } else {
        owned_natures = all_natures().to_vec();
        owned_natures.as_slice()
    };
    let result = hp_def_survival_search_from_hp_percent(
        data,
        &benchmark,
        natures,
        request.max_ko_chance,
        request.hp_percent.unwrap_or(100.0),
        request.limit,
    )?;
    Ok(HpDefSurvivalResponse {
        best: result.matches.first().cloned(),
        matches: result.matches,
        closest_miss: result.closest_miss,
    })
}

pub fn run_defensive_optimization(request: OptimizeRequest) -> Result<Vec<RankedSpread>, ApiError> {
    let data = ChampionsData::load()?;
    run_defensive_optimization_with_data(&data, request)
}

pub fn run_defensive_optimization_with_data(
    data: &ChampionsData,
    request: OptimizeRequest,
) -> Result<Vec<RankedSpread>, ApiError> {
    let benchmarks = optimize_benchmarks_from_request(request.benchmarks)?;
    let search = spread_search_from_request(request.full_spend, request.locked);
    optimize_defensive(data, &benchmarks, search, default_limit(request.limit)).map_err(Into::into)
}

pub fn run_offensive_optimization(request: OptimizeRequest) -> Result<Vec<RankedSpread>, ApiError> {
    let data = ChampionsData::load()?;
    run_offensive_optimization_with_data(&data, request)
}

pub fn run_offensive_optimization_with_data(
    data: &ChampionsData,
    request: OptimizeRequest,
) -> Result<Vec<RankedSpread>, ApiError> {
    let benchmarks = optimize_benchmarks_from_request(request.benchmarks)?;
    let search = spread_search_from_request(request.full_spend, request.locked);
    optimize_offensive(data, &benchmarks, search, default_limit(request.limit)).map_err(Into::into)
}

pub fn find_min_combined_hp_def_survival(
    request: CombinedHpDefSurvivalRequest,
) -> Result<CombinedHpDefSurvivalResponse, ApiError> {
    let data = ChampionsData::load()?;
    find_min_combined_hp_def_survival_with_data(&data, request)
}

pub fn find_min_combined_hp_def_survival_with_data(
    data: &ChampionsData,
    request: CombinedHpDefSurvivalRequest,
) -> Result<CombinedHpDefSurvivalResponse, ApiError> {
    let limit = if request.limit == 0 {
        10
    } else {
        request.limit
    };
    let defender = parse_set(&request.defender_set)?;
    let benchmarks = request
        .hits
        .into_iter()
        .map(|hit| {
            let mut benchmark = DamageBenchmark::new(
                parse_set(&hit.attacker_set)?,
                defender.clone(),
                hit.move_name,
            );
            benchmark.move_times_affected = hit.move_times_affected;
            if let Some(field) = hit.field {
                benchmark.fairy_aura = field.fairy_aura;
                benchmark.field = field.into_field();
            }
            Ok(benchmark)
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    let owned_natures;
    let natures = if let Some(nature) = request.nature {
        owned_natures = vec![nature];
        owned_natures.as_slice()
    } else if request.optimize_nature {
        owned_natures = optimized_combined_defensive_natures(data, &benchmarks)?;
        owned_natures.as_slice()
    } else {
        owned_natures = all_natures().to_vec();
        owned_natures.as_slice()
    };
    let result = hp_def_combined_survival_search(
        data,
        &benchmarks,
        natures,
        request.max_ko_chance,
        request.hp_percent.unwrap_or(100.0),
        limit,
    )?;
    Ok(CombinedHpDefSurvivalResponse {
        best: result.matches.first().cloned(),
        matches: result.matches,
        closest_miss: result.closest_miss,
    })
}

pub fn find_min_offensive_ko(request: OffensiveKoRequest) -> Result<OffensiveKoResponse, ApiError> {
    let data = ChampionsData::load()?;
    find_min_offensive_ko_with_data(&data, request)
}

pub fn find_min_offensive_ko_with_data(
    data: &ChampionsData,
    request: OffensiveKoRequest,
) -> Result<OffensiveKoResponse, ApiError> {
    let limit = if request.limit == 0 {
        10
    } else {
        request.limit
    };
    let benchmark = benchmark_from_sets(
        &request.attacker_set,
        &request.defender_set,
        request.move_name,
        request.move_times_affected,
        request.field,
    )?;
    let owned_natures;
    let natures = if let Some(nature) = request.nature {
        owned_natures = vec![nature];
        owned_natures.as_slice()
    } else if request.optimize_nature {
        owned_natures = optimized_offensive_natures(data, &benchmark)?.to_vec();
        owned_natures.as_slice()
    } else {
        owned_natures = all_natures().to_vec();
        owned_natures.as_slice()
    };
    let result = offensive_ko_search(data, &benchmark, natures, request.min_ko_chance, limit)?;
    Ok(OffensiveKoResponse {
        best: result.matches.first().cloned(),
        matches: result.matches,
        closest_miss: result.closest_miss,
    })
}

fn benchmark_from_sets(
    attacker_set: &str,
    defender_set: &str,
    move_name: String,
    move_times_affected: u8,
    field: Option<FieldRequest>,
) -> Result<DamageBenchmark, ApiError> {
    let mut benchmark = DamageBenchmark::new(
        parse_set(attacker_set)?,
        parse_set(defender_set)?,
        move_name,
    );
    benchmark.move_times_affected = move_times_affected;
    if let Some(field) = field {
        benchmark.fairy_aura = field.fairy_aura;
        benchmark.field = field.into_field();
    }
    Ok(benchmark)
}

fn optimize_benchmarks_from_request(
    benchmarks: Vec<OptimizeBenchmarkRequest>,
) -> Result<Vec<DamageBenchmark>, ApiError> {
    benchmarks
        .into_iter()
        .map(|benchmark| {
            benchmark_from_sets(
                &benchmark.attacker_set,
                &benchmark.defender_set,
                benchmark.move_name,
                benchmark.move_times_affected,
                benchmark.field,
            )
        })
        .collect()
}

impl FieldRequest {
    pub fn into_field(self) -> Field {
        let mut field = Field {
            format: self.format.unwrap_or(Format::Doubles),
            weather: self.weather.unwrap_or(Weather::None),
            terrain: self.terrain.unwrap_or(Terrain::None),
            gravity: self.gravity,
            protect: self.protect,
            helping_hand: self.helping_hand,
            attacker_tailwind: self.attacker_tailwind,
            defender_tailwind: self.defender_tailwind,
            defender_side: SideConditions {
                reflect: self.defender_reflect,
                light_screen: self.defender_light_screen,
                aurora_veil: self.defender_aurora_veil,
                friend_guard: self.defender_friend_guard,
            },
            ..Field::default()
        };
        if field.format == Format::Singles {
            field.helping_hand = false;
        }
        field
    }
}

fn spread_search_from_request(full_spend: bool, locked: LockedStatsRequest) -> SpreadSearch {
    let mut search = if full_spend {
        SpreadSearch::full_spend()
    } else {
        SpreadSearch::all_legal()
    };
    search.locked = LockedStats {
        hp: locked.hp,
        attack: locked.attack,
        defense: locked.defense,
        special_attack: locked.special_attack,
        special_defense: locked.special_defense,
        speed: locked.speed,
    };
    search
}

fn default_limit(limit: usize) -> usize {
    if limit == 0 {
        10
    } else {
        limit
    }
}

fn sort_unique(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

impl From<DamageResult> for DamageResponse {
    fn from(value: DamageResult) -> Self {
        Self {
            rolls: value.damage_rolls.clone(),
            summary: DamageSummary::from(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KINGAMBIT: &str = "Kingambit\nAbility: Defiant\nSPs: 32 Atk\nAdamant Nature\n- Iron Head";
    const FLOETTE: &str = "Mega Floette\n- Protect";

    #[test]
    fn finds_min_survival_spread_for_visualizer() {
        let data = ChampionsData::load().unwrap();
        let response = find_min_hp_def_survival_with_data(
            &data,
            HpDefSurvivalRequest {
                attacker_set: KINGAMBIT.to_owned(),
                defender_set: FLOETTE.to_owned(),
                move_name: "Iron Head".to_owned(),
                max_ko_chance: 0.125,
                hp_percent: None,
                nature: None,
                optimize_nature: false,
                limit: 4,
                move_times_affected: 0,
                field: None,
            },
        )
        .unwrap();

        let best = response.best.unwrap();
        assert_eq!(best.total_points, 24);
        assert_eq!(best.sps.hp, 4);
        assert_eq!(best.sps.defense, 20);
        assert_eq!(best.result.max_damage, 158);
        assert_eq!(best.result.ko_chance, Some(0.125));
    }

    #[test]
    fn applies_field_request_to_damage_calculation() {
        let data = ChampionsData::load().unwrap();
        let open = calculate_damage_request_with_data(
            &data,
            DamageRequest {
                attacker_set: KINGAMBIT.to_owned(),
                defender_set: FLOETTE.to_owned(),
                move_name: "Iron Head".to_owned(),
                move_times_affected: 0,
                field: None,
            },
        )
        .unwrap();
        let reflected = calculate_damage_request_with_data(
            &data,
            DamageRequest {
                attacker_set: KINGAMBIT.to_owned(),
                defender_set: FLOETTE.to_owned(),
                move_name: "Iron Head".to_owned(),
                move_times_affected: 0,
                field: Some(FieldRequest {
                    defender_reflect: true,
                    ..FieldRequest::default()
                }),
            },
        )
        .unwrap();

        assert!(reflected.summary.max_damage < open.summary.max_damage);
    }

    #[test]
    fn finds_modest_min_survival_spread_for_visualizer() {
        let data = ChampionsData::load().unwrap();
        let response = find_min_hp_def_survival_with_data(
            &data,
            HpDefSurvivalRequest {
                attacker_set: KINGAMBIT.to_owned(),
                defender_set: FLOETTE.to_owned(),
                move_name: "Iron Head".to_owned(),
                max_ko_chance: 0.125,
                hp_percent: None,
                nature: Some(Nature::Modest),
                optimize_nature: false,
                limit: 1,
                move_times_affected: 0,
                field: None,
            },
        )
        .unwrap();

        let best = response.best.unwrap();
        assert_eq!(best.nature, Nature::Modest);
        assert_eq!(best.total_points, 36);
        assert_eq!(best.sps.hp, 4);
        assert_eq!(best.sps.defense, 32);
    }

    #[test]
    fn finds_combined_survival_spread_for_visualizer() {
        let data = ChampionsData::load().unwrap();
        let response = find_min_combined_hp_def_survival_with_data(
            &data,
            CombinedHpDefSurvivalRequest {
                defender_set: FLOETTE.to_owned(),
                hits: vec![
                    IncomingHitRequest {
                        attacker_set: KINGAMBIT.to_owned(),
                        move_name: "Iron Head".to_owned(),
                        move_times_affected: 0,
                        field: None,
                    },
                    IncomingHitRequest {
                        attacker_set: KINGAMBIT.to_owned(),
                        move_name: "Iron Head".to_owned(),
                        move_times_affected: 0,
                        field: None,
                    },
                ],
                max_ko_chance: 1.0,
                hp_percent: Some(50.0),
                nature: Some(Nature::Bold),
                optimize_nature: false,
                limit: 1,
            },
        )
        .unwrap();

        let best = response.best.unwrap();
        assert_eq!(best.hits.len(), 2);
        assert_eq!(
            best.combined.starting_hp,
            (best.final_stats.hp as f32 * 0.5).ceil() as u16
        );
        assert!(best.combined.ko_chance > 0.0);
    }

    #[test]
    fn finds_min_offensive_ko_for_visualizer() {
        let data = ChampionsData::load().unwrap();
        let response = find_min_offensive_ko_with_data(
            &data,
            OffensiveKoRequest {
                attacker_set: "Basculegion (Male)\nAbility: Adaptability\n- Last Respects"
                    .to_owned(),
                defender_set: "Aegislash (Shield Forme)\nSPs: 30 HP / 4 Def\n- Protect".to_owned(),
                move_name: "Last Respects".to_owned(),
                min_ko_chance: 1.0,
                nature: None,
                optimize_nature: false,
                limit: 1,
                move_times_affected: 1,
                field: None,
            },
        )
        .unwrap();

        let best = response.best.unwrap();
        assert_eq!(best.result.ko_chance, Some(1.0));
    }

    #[test]
    fn optimized_nature_mode_only_checks_boosted_and_neutral() {
        let data = ChampionsData::load().unwrap();
        let response = find_min_offensive_ko_with_data(
            &data,
            OffensiveKoRequest {
                attacker_set: "Basculegion (Male)\nAbility: Adaptability\n- Last Respects"
                    .to_owned(),
                defender_set: "Aegislash (Shield Forme)\nSPs: 30 HP / 4 Def\n- Protect".to_owned(),
                move_name: "Last Respects".to_owned(),
                min_ko_chance: 1.0,
                nature: None,
                optimize_nature: true,
                limit: 10,
                move_times_affected: 1,
                field: None,
            },
        )
        .unwrap();

        assert!(response
            .matches
            .iter()
            .all(|spread| matches!(spread.nature, Nature::Adamant | Nature::Hardy)));
    }
}
