use crate::data::{ChampionsData, DataError};
use crate::showdown::{ParsedSet, RivalryMode};
use damage_calc::{
    calculate_damage, Ability, Boosts, CalcInput, DamageResult, Field, Format, Pokemon, Ruleset,
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
    pub attacker_boosts: Option<Boosts>,
    pub defender_boosts: Option<Boosts>,
    pub defender_current_hp: Option<u16>,
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
            attacker_boosts: None,
            defender_boosts: None,
            defender_current_hp: None,
        }
    }
}

pub fn calculate_benchmark(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
) -> Result<DamageResult, BridgeError> {
    let mut attacker = build_pokemon(data, &benchmark.attacker)?;
    let mut defender = build_pokemon(data, &benchmark.defender)?;
    let mut move_ = data.move_data(&benchmark.move_name)?.to_damage_move()?;
    move_.times_affected = benchmark.move_times_affected;
    move_.targets_single_target =
        move_.targets_single_target || benchmark.attacker.move_targets_single_target;
    if let Some(hits) = fixed_hit_count(&move_.name) {
        move_.hits = hits;
    }
    if is_slice_move(&move_.name) {
        move_.is_slice = true;
    }
    if benchmark.fairy_aura {
        attacker.ability = Ability::FairyAura;
    }
    if attacker.ability == Ability::SkillLink && is_skill_link_move(&move_.name) {
        move_.hits = 5;
    }
    if let Some(boosts) = benchmark.attacker_boosts {
        attacker.boosts = boosts;
    }
    if let Some(boosts) = benchmark.defender_boosts {
        defender.boosts = boosts;
    }
    if let Some(current_hp) = benchmark.defender_current_hp {
        defender.current_hp = Some(current_hp);
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
    pokemon.status = set.status;
    pokemon.ability_on = set.ability_on;
    pokemon.supreme_overlord_allies = set.supreme_overlord_allies;
    if pokemon.ability == Ability::Rivalry {
        match set.rivalry {
            Some(RivalryMode::SameGender) => pokemon.custom_bp_mods.push(5120),
            Some(RivalryMode::OppositeGender) => pokemon.custom_bp_mods.push(3072),
            None => {}
        }
    }
    Ok(pokemon)
}

fn is_skill_link_move(name: &str) -> bool {
    matches!(
        name,
        "Arm Thrust"
            | "Barrage"
            | "Bone Rush"
            | "Bullet Seed"
            | "Comet Punch"
            | "Double Slap"
            | "Fury Attack"
            | "Fury Swipes"
            | "Icicle Spear"
            | "Pin Missile"
            | "Rock Blast"
            | "Scale Shot"
            | "Spike Cannon"
            | "Tail Slap"
            | "Water Shuriken"
    )
}

fn fixed_hit_count(name: &str) -> Option<u8> {
    match name {
        "Double Hit" | "Double Iron Bash" | "Double Kick" | "Double Shock" | "Dual Chop"
        | "Dual Wingbeat" | "Gear Grind" | "Tachyon Cutter" | "Twin Beam" | "Twinneedle" => Some(2),
        "Surging Strikes" | "Triple Axel" | "Triple Dive" | "Triple Kick" => Some(3),
        _ => None,
    }
}

fn is_slice_move(name: &str) -> bool {
    matches!(
        name,
        "Air Cutter"
            | "Aqua Cutter"
            | "Behemoth Blade"
            | "Bitter Blade"
            | "Ceaseless Edge"
            | "Cut"
            | "Fury Cutter"
            | "Kowtow Cleave"
            | "Leaf Blade"
            | "Night Slash"
            | "Psyblade"
            | "Psycho Cut"
            | "Razor Leaf"
            | "Razor Shell"
            | "Sacred Sword"
            | "Secret Sword"
            | "Slash"
            | "Solar Blade"
            | "Stone Axe"
            | "X-Scissor"
    )
}
