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
use damage_calc::{Boosts, DamageResult, Field, Format, Nature, SideConditions, Terrain, Weather};
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
    #[serde(default)]
    pub attacker_boosts: BoostsRequest,
    #[serde(default)]
    pub defender_boosts: BoostsRequest,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct BoostsRequest {
    #[serde(default)]
    pub attack: i8,
    #[serde(default)]
    pub defense: i8,
    #[serde(default)]
    pub special_attack: i8,
    #[serde(default)]
    pub special_defense: i8,
    #[serde(default)]
    pub speed: i8,
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
                benchmark.attacker_boosts = Some(field.attacker_boosts.into_boosts());
                benchmark.defender_boosts = Some(field.defender_boosts.into_boosts());
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
        benchmark.attacker_boosts = Some(field.attacker_boosts.into_boosts());
        benchmark.defender_boosts = Some(field.defender_boosts.into_boosts());
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

impl BoostsRequest {
    pub fn into_boosts(self) -> Boosts {
        Boosts {
            attack: self.attack.clamp(-6, 6),
            defense: self.defense.clamp(-6, 6),
            special_attack: self.special_attack.clamp(-6, 6),
            special_defense: self.special_defense.clamp(-6, 6),
            speed: self.speed.clamp(-6, 6),
        }
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
    use std::collections::BTreeSet;

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
    fn calculates_sneasler_close_combat_into_chople_kingambit() {
        let data = ChampionsData::load().unwrap();
        let response = calculate_damage_request_with_data(
            &data,
            DamageRequest {
                attacker_set:
                    "Sneasler @ White Herb\nAbility: Unburden\nSPs: 32 Atk\nAdamant Nature\n- Close Combat"
                        .to_owned(),
                defender_set: "Kingambit @ Chople Berry\nSPs: 2 HP / 9 Def\n- Protect".to_owned(),
                move_name: "Close Combat".to_owned(),
                move_times_affected: 0,
                field: None,
            },
        )
        .unwrap();

        assert_eq!(
            response.rolls,
            vec![182, 182, 186, 188, 192, 192, 194, 198, 198, 200, 204, 206, 206, 210, 212, 216]
        );
        assert_eq!(response.summary.ko_chance, Some(1.0));
    }

    #[test]
    fn calculates_damage_calc_suffixed_sneasler_close_combat() {
        let data = ChampionsData::load().unwrap();
        let response = calculate_damage_request_with_data(
            &data,
            DamageRequest {
                attacker_set:
                    "Sneasler @ White Herb\nAbility: Unburden\nSPs: 10+ Atk\n- Close Combat"
                        .to_owned(),
                defender_set: "Kingambit @ Chople Berry\nSPs: 0 HP / 9 Def\n- Protect".to_owned(),
                move_name: "Close Combat".to_owned(),
                move_times_affected: 0,
                field: None,
            },
        )
        .unwrap();

        assert_eq!(
            response.rolls,
            vec![162, 164, 164, 168, 168, 170, 174, 174, 176, 180, 180, 182, 186, 186, 188, 192]
        );
        assert_eq!(response.summary.ko_chance, Some(0.5));
    }

    #[test]
    fn defensive_min_uses_suffixed_attacker_nature() {
        let data = ChampionsData::load().unwrap();
        let response = find_min_hp_def_survival_with_data(
            &data,
            HpDefSurvivalRequest {
                attacker_set:
                    "Sneasler @ White Herb\nAbility: Unburden\nSPs: 10+ Atk\n- Close Combat"
                        .to_owned(),
                defender_set: "Kingambit @ Chople Berry\n- Protect".to_owned(),
                move_name: "Close Combat".to_owned(),
                max_ko_chance: 0.125,
                hp_percent: None,
                nature: Some(Nature::Adamant),
                optimize_nature: false,
                limit: 10,
                move_times_affected: 0,
                field: None,
            },
        )
        .unwrap();

        assert!(response
            .matches
            .iter()
            .all(|spread| spread.total_points > 9));
    }

    #[test]
    fn defensive_min_uses_showdown_attacker_nature_line() {
        let data = ChampionsData::load().unwrap();
        let response = find_min_hp_def_survival_with_data(
            &data,
            HpDefSurvivalRequest {
                attacker_set:
                    "Sneasler @ White Herb\nAbility: Unburden\nSPs: 10 Atk\nAdamant Nature\n- Close Combat"
                        .to_owned(),
                defender_set: "Kingambit @ Chople Berry\n- Protect".to_owned(),
                move_name: "Close Combat".to_owned(),
                max_ko_chance: 0.125,
                hp_percent: None,
                nature: Some(Nature::Adamant),
                optimize_nature: false,
                limit: 10,
                move_times_affected: 0,
                field: None,
            },
        )
        .unwrap();

        assert!(response
            .matches
            .iter()
            .all(|spread| spread.total_points > 9));
    }

    #[test]
    fn defensive_min_treats_low_evs_as_champions_points() {
        let data = ChampionsData::load().unwrap();
        let response = find_min_hp_def_survival_with_data(
            &data,
            HpDefSurvivalRequest {
                attacker_set: "Sneasler @ White Herb\nAbility: Unburden\nLevel: 50\nEVs: 20 HP / 10 Atk / 21 Def / 15 Spe\nAdamant Nature\n- Close Combat\n- Fake Out\n- Dire Claw\n- Protect"
                    .to_owned(),
                defender_set: "Kingambit @ Chople Berry\nAbility: Defiant\nSPs: 32 Atk\nAdamant Nature\n- Iron Head\n- Kowtow Cleave"
                    .to_owned(),
                move_name: "Close Combat".to_owned(),
                max_ko_chance: 0.125,
                hp_percent: None,
                nature: Some(Nature::Adamant),
                optimize_nature: false,
                limit: 10,
                move_times_affected: 0,
                field: None,
            },
        )
        .unwrap();

        assert!(response
            .matches
            .iter()
            .all(|spread| spread.total_points > 9));
    }

    #[test]
    fn damage_benchmark_matches_known_calcs() {
        let data = ChampionsData::load().unwrap();
        let cases = [
            BenchmarkCase {
                name: "Sneasler Close Combat vs Chople Kingambit",
                attacker: "Sneasler @ White Herb\nAbility: Unburden\nSPs: 32+ Atk\n- Close Combat",
                defender: "Kingambit @ Chople Berry\nSPs: 2 HP / 9 Def\n- Protect",
                move_name: "Close Combat",
                field: None,
                expected_min: 182,
                expected_max: 216,
                expected_unique: &[182, 186, 188, 192, 194, 198, 200, 204, 206, 210, 212, 216],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Kingambit Iron Head vs max Mega Floette",
                attacker:
                    "Kingambit @ Black Glasses\nAbility: Defiant\nSPs: 32+ Atk\n- Iron Head",
                defender: "Mega Floette\nSPs: 32 HP / 32 Def\n- Protect",
                move_name: "Iron Head",
                field: None,
                expected_min: 134,
                expected_max: 158,
                expected_unique: &[134, 138, 140, 144, 146, 150, 152, 156, 158],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Magnet Transistor Pikachu Thunderbolt in Electric Terrain",
                attacker: "Pikachu @ Magnet\nAbility: Transistor\n- Thunderbolt",
                defender: "Milotic\n- Protect",
                move_name: "Thunderbolt",
                field: Some(FieldRequest {
                    terrain: Some(Terrain::Electric),
                    ..FieldRequest::default()
                }),
                expected_min: 102,
                expected_max: 120,
                expected_unique: &[102, 104, 108, 110, 114, 116, 120],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Charizard Rock Slide spread in Doubles",
                attacker: "Charizard\n- Rock Slide",
                defender: "Volcarona\n- Protect",
                move_name: "Rock Slide",
                field: None,
                expected_min: 104,
                expected_max: 124,
                expected_unique: &[104, 108, 112, 116, 120, 124],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Charizard Rock Slide single target in Doubles",
                attacker: "Charizard\nTarget: single\n- Rock Slide",
                defender: "Volcarona\n- Protect",
                move_name: "Rock Slide",
                field: None,
                expected_min: 140,
                expected_max: 168,
                expected_unique: &[140, 144, 148, 152, 156, 160, 164, 168],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Burned Machamp Drain Punch through Reflect",
                attacker: "Machamp\nStatus: Burned\n- Drain Punch",
                defender: "Snorlax\n- Protect",
                move_name: "Drain Punch",
                field: Some(FieldRequest {
                    defender_reflect: true,
                    ..FieldRequest::default()
                }),
                expected_min: 51,
                expected_max: 60,
                expected_unique: &[51, 52, 53, 54, 55, 56, 57, 58, 59, 60],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Ninetales Flamethrower in Sun",
                attacker: "Ninetales\n- Flamethrower",
                defender: "Scizor\n- Protect",
                move_name: "Flamethrower",
                field: Some(FieldRequest {
                    weather: Some(Weather::Sun),
                    ..FieldRequest::default()
                }),
                expected_min: 304,
                expected_max: 364,
                expected_unique: &[304, 312, 316, 324, 328, 336, 340, 348, 352, 360, 364],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Pelipper Weather Ball in Rain",
                attacker: "Pelipper\n- Weather Ball",
                defender: "Camerupt\n- Protect",
                move_name: "Weather Ball",
                field: Some(FieldRequest {
                    weather: Some(Weather::Rain),
                    ..FieldRequest::default()
                }),
                expected_min: 412,
                expected_max: 492,
                expected_unique: &[
                    412, 420, 424, 432, 436, 444, 448, 456, 460, 468, 472, 480, 484, 492,
                ],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Abomasnow Blizzard into active Multiscale Dragonite",
                attacker: "Abomasnow\n- Blizzard",
                defender: "Dragonite\nAbility: Multiscale\nAbility On: true\n- Protect",
                move_name: "Blizzard",
                field: None,
                expected_min: 86,
                expected_max: 104,
                expected_unique: &[86, 90, 92, 96, 98, 102, 104],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Gyarados Waterfall into Passho Torkoal",
                attacker: "Gyarados\n- Waterfall",
                defender: "Torkoal @ Passho Berry\n- Protect",
                move_name: "Waterfall",
                field: None,
                expected_min: 42,
                expected_max: 49,
                expected_unique: &[42, 43, 45, 46, 48, 49],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Gengar Shadow Ball into Kasib Clefable",
                attacker: "Gengar\n- Shadow Ball",
                defender: "Clefable @ Kasib Berry\n- Protect",
                move_name: "Shadow Ball",
                field: None,
                expected_min: 63,
                expected_max: 75,
                expected_unique: &[63, 64, 66, 67, 69, 70, 72, 73, 75],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Mega Kangaskhan Parental Bond Double-Edge",
                attacker: "Mega Kangaskhan\nAbility: Parental Bond\n- Double-Edge",
                defender: "Blastoise\n- Protect",
                move_name: "Double-Edge",
                field: None,
                expected_min: 123,
                expected_max: 145,
                expected_unique: &[
                    123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136,
                    137, 138, 139, 140, 141, 142, 143, 144, 145,
                ],
                expected_roll_count: Some(256),
            },
            BenchmarkCase {
                name: "Skill Link Toucannon Bullet Seed",
                attacker: "Toucannon\nAbility: Skill Link\n- Bullet Seed",
                defender: "Slowbro\n- Protect",
                move_name: "Bullet Seed",
                field: None,
                expected_min: 110,
                expected_max: 130,
                expected_unique: &[110, 112, 114, 116, 118, 120, 122, 124, 126, 128, 130],
                expected_roll_count: Some(1_048_576),
            },
            BenchmarkCase {
                name: "Swift Swim Beartic Electro Ball in Rain",
                attacker: "Beartic\nAbility: Swift Swim\n- Electro Ball",
                defender: "Pelipper\n- Protect",
                move_name: "Electro Ball",
                field: Some(FieldRequest {
                    weather: Some(Weather::Rain),
                    ..FieldRequest::default()
                }),
                expected_min: 92,
                expected_max: 112,
                expected_unique: &[92, 96, 100, 104, 108, 112],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Analytic Starmie Psychic moving last",
                attacker: "Starmie\nAbility: Analytic\n- Psychic",
                defender: "Venusaur\n- Protect",
                move_name: "Psychic",
                field: Some(FieldRequest {
                    defender_tailwind: true,
                    ..FieldRequest::default()
                }),
                expected_min: 134,
                expected_max: 158,
                expected_unique: &[134, 138, 140, 144, 146, 150, 152, 156, 158],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Rivalry Luxray same gender",
                attacker: "Luxray\nAbility: Rivalry\nRivalry: same\n- Wild Charge",
                defender: "Pelipper\n- Protect",
                move_name: "Wild Charge",
                field: None,
                expected_min: 300,
                expected_max: 352,
                expected_unique: &[300, 304, 312, 316, 324, 328, 336, 340, 348, 352],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Rivalry Luxray opposite gender",
                attacker: "Luxray\nAbility: Rivalry\nRivalry: opposite\n- Wild Charge",
                defender: "Pelipper\n- Protect",
                move_name: "Wild Charge",
                field: None,
                expected_min: 180,
                expected_max: 216,
                expected_unique: &[180, 184, 192, 196, 204, 208, 216],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Fairy Aura Mega Floette Moonblast",
                attacker: "Mega Floette\nAbility: Fairy Aura\n- Moonblast",
                defender: "Hydreigon\n- Protect",
                move_name: "Moonblast",
                field: None,
                expected_min: 456,
                expected_max: 540,
                expected_unique: &[
                    456, 460, 468, 472, 480, 484, 492, 496, 504, 508, 516, 520, 528, 532,
                    540,
                ],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Sharpness Gallade Psycho Cut",
                attacker: "Gallade\nAbility: Sharpness\n- Psycho Cut",
                defender: "Toxapex\n- Protect",
                move_name: "Psycho Cut",
                field: None,
                expected_min: 102,
                expected_max: 120,
                expected_unique: &[102, 104, 108, 110, 114, 116, 120],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Supreme Overlord Kingambit with 3 fainted allies",
                attacker:
                    "Kingambit\nAbility: Supreme Overlord\nSupreme Overlord Allies: 3\n- Kowtow Cleave",
                defender: "Gengar\n- Protect",
                move_name: "Kowtow Cleave",
                field: None,
                expected_min: 242,
                expected_max: 288,
                expected_unique: &[
                    242, 246, 248, 252, 254, 258, 260, 264, 266, 270, 272, 276, 278, 282,
                    284, 288,
                ],
                expected_roll_count: None,
            },
        ];

        for case in cases {
            assert_benchmark_case(&data, case);
        }
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

    #[derive(Clone, Copy)]
    struct BenchmarkCase {
        name: &'static str,
        attacker: &'static str,
        defender: &'static str,
        move_name: &'static str,
        field: Option<FieldRequest>,
        expected_min: u16,
        expected_max: u16,
        expected_unique: &'static [u16],
        expected_roll_count: Option<usize>,
    }

    fn assert_benchmark_case(data: &ChampionsData, case: BenchmarkCase) {
        let response = calculate_damage_request_with_data(
            data,
            DamageRequest {
                attacker_set: case.attacker.to_owned(),
                defender_set: case.defender.to_owned(),
                move_name: case.move_name.to_owned(),
                move_times_affected: 0,
                field: case.field,
            },
        )
        .unwrap_or_else(|error| panic!("{} failed: {error}", case.name));
        assert_eq!(
            response.summary.min_damage, case.expected_min,
            "{} min damage",
            case.name
        );
        assert_eq!(
            response.summary.max_damage, case.expected_max,
            "{} max damage",
            case.name
        );
        let unique = response
            .rolls
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        assert_eq!(unique, case.expected_unique, "{} unique rolls", case.name);
        if let Some(expected_roll_count) = case.expected_roll_count {
            assert_eq!(
                response.rolls.len(),
                expected_roll_count,
                "{} roll count",
                case.name
            );
        }
    }
}
