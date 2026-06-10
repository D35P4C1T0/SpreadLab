use crate::data::{ChampionsData, DataError};
use crate::showdown::ParsedSet;
use damage_calc::{
    calculate_damage, Ability, CalcInput, DamageResult, Field, Format, Pokemon, Ruleset,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error(transparent)]
    Data(#[from] DataError),
    #[error("damage calculation failed: {0}")]
    Damage(String),
}

#[derive(Debug, Clone)]
pub struct DamageBenchmark {
    pub attacker: ParsedSet,
    pub defender: ParsedSet,
    pub move_name: String,
    pub move_times_affected: u8,
    pub field: Field,
    pub fairy_aura: bool,
}

impl DamageBenchmark {
    pub fn new(attacker: ParsedSet, defender: ParsedSet, move_name: impl Into<String>) -> Self {
        let field = Field {
            format: Format::Doubles,
            ..Field::default()
        };
        Self {
            attacker,
            defender,
            move_name: move_name.into(),
            move_times_affected: 0,
            field,
            fairy_aura: false,
        }
    }
}

pub fn calculate_benchmark(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
) -> Result<DamageResult, BridgeError> {
    let mut attacker = build_pokemon(data, &benchmark.attacker)?;
    let defender = build_pokemon(data, &benchmark.defender)?;
    let mut move_ = data.move_data(&benchmark.move_name)?.to_damage_move()?;
    move_.times_affected = benchmark.move_times_affected;
    if benchmark.fairy_aura {
        attacker.ability = Ability::FairyAura;
    }
    calculate_damage(CalcInput {
        attacker,
        defender,
        move_,
        field: benchmark.field,
        ruleset: Ruleset::Champions,
    })
    .map_err(|error| BridgeError::Damage(error.to_string()))
}

pub fn build_pokemon(data: &ChampionsData, set: &ParsedSet) -> Result<Pokemon, BridgeError> {
    let species = data.species(&set.species)?;
    let mut pokemon = Pokemon::champions(
        species.display_name.clone(),
        species.damage_types()?,
        species.base_stats().into(),
        set.stat_points.into(),
        set.nature,
    );
    pokemon.weight_kg = species.weight_kg;
    if let Some(ability) = &set.ability {
        pokemon.ability = crate::data::parse_ability(ability)?;
    }
    if let Some(item) = &set.item {
        pokemon.item = crate::data::parse_item(item)?;
    }
    if let Some(tera_type) = &set.tera_type {
        pokemon.tera_type = Some(crate::data::parse_type(tera_type)?);
    }
    Ok(pokemon)
}
