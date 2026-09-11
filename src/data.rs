use crate::showdown::parse_nature_name;
use crate::stats::BaseStats;
use damage_calc::data::champions::{
    CHAMPIONS_ABILITIES, CHAMPIONS_ITEM_VALUES, CHAMPIONS_SPECIES, REGULATION_M_B_POKEMON,
    REGULATION_M_C_POKEMON,
};
use damage_calc::{Ability, Category, Item, Move, PokemonType};
use serde::Deserialize;
use std::collections::HashMap;
use thiserror::Error;

pub const POKEMON_CHAMPIONS_ITEMS: &[&str] = damage_calc::data::champions::CHAMPIONS_ITEMS;

#[derive(Debug, Error)]
pub enum DataError {
    #[error("failed to parse Champions data: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("unknown species: {0}")]
    UnknownSpecies(String),
    #[error("unknown move: {0}")]
    UnknownMove(String),
    #[error("unknown Pokemon type: {0}")]
    UnknownType(String),
    #[error("unknown move category: {0}")]
    UnknownCategory(String),
    #[error("unknown ability: {0}")]
    UnknownAbility(String),
    #[error("unknown item: {0}")]
    UnknownItem(String),
    #[error("unknown nature in spread: {0}")]
    UnknownNature(String),
    #[error("malformed spread: {0}")]
    MalformedSpread(String),
}

#[derive(Debug, Clone)]
pub struct ChampionsData {
    species_by_key: HashMap<String, SpeciesData>,
    moves_by_key: HashMap<String, MoveData>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeciesData {
    pub display_name: String,
    #[serde(default)]
    pub types: Vec<String>,
    pub base_stats: BaseStatsJson,
    #[serde(default)]
    pub weight_kg: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseStatsJson {
    pub hp: u16,
    pub attack: u16,
    pub defense: u16,
    pub special_attack: u16,
    pub special_defense: u16,
    pub speed: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveData {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub category: String,
    #[serde(default)]
    pub power: u16,
    #[serde(default)]
    pub makes_contact: bool,
    #[serde(default)]
    pub priority: i8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChampionsDataJson {
    species: Vec<SpeciesData>,
    moves: Vec<MoveData>,
}

impl ChampionsData {
    pub fn load() -> Result<Self, DataError> {
        let parsed: ChampionsDataJson =
            serde_json::from_str(damage_calc::data::CHAMPIONS_DATA_JSON)?;
        let species_by_key = parsed
            .species
            .into_iter()
            .flat_map(|species| {
                let mut keys = vec![normalize_name(&species.display_name)];
                keys.push(normalize_name(
                    &species.display_name.replace("-Mega", " Mega"),
                ));
                if let Some(mega_suffix) = species.display_name.strip_prefix("Mega ") {
                    let parts = mega_suffix.split_whitespace().collect::<Vec<_>>();
                    if parts.len() >= 2 {
                        let stone_suffix = parts.last().copied().unwrap_or_default();
                        let base = parts[..parts.len() - 1].join(" ");
                        keys.push(normalize_name(&format!("{base} Mega {stone_suffix}")));
                        keys.push(normalize_name(&format!("{base}-Mega-{stone_suffix}")));
                    } else {
                        keys.push(normalize_name(&format!("{mega_suffix} Mega")));
                    }
                }
                let aliases: &[&str] = match species.display_name.as_str() {
                    "Persian (Alolan)" => &["Persian-Alola"],
                    "Toxtricity (Amped Form)" => &["Toxtricity", "Toxtricity-Amped"],
                    "Toxtricity (Low Key Form)" => &["Toxtricity-Low-Key"],
                    "Indeedee (Male)" => &["Indeedee", "Indeedee-M"],
                    "Indeedee (Female)" => &["Indeedee-F"],
                    "Squawkabilly (Green Plumage)" => &["Squawkabilly", "Squawkabilly-Green"],
                    "Squawkabilly (Blue Plumage)" => &["Squawkabilly-Blue"],
                    "Squawkabilly (Yellow Plumage)" => &["Squawkabilly-Yellow"],
                    "Squawkabilly (White Plumage)" => &["Squawkabilly-White"],
                    _ => &[],
                };
                keys.extend(aliases.iter().map(|name| normalize_name(name)));
                keys.into_iter()
                    .map(move |key| (key, species.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();
        let moves_by_key = parsed
            .moves
            .into_iter()
            .map(|move_| (normalize_name(&move_.name), move_))
            .collect();

        Ok(Self {
            species_by_key,
            moves_by_key,
        })
    }

    pub fn species(&self, name: &str) -> Result<&SpeciesData, DataError> {
        self.species_by_key
            .get(&normalize_name(name))
            .ok_or_else(|| DataError::UnknownSpecies(name.to_owned()))
    }

    pub fn move_data(&self, name: &str) -> Result<&MoveData, DataError> {
        self.moves_by_key
            .get(&normalize_name(name))
            .ok_or_else(|| DataError::UnknownMove(name.to_owned()))
    }

    pub fn species_names(&self) -> impl Iterator<Item = &'static str> {
        CHAMPIONS_SPECIES.iter().map(|species| species.display_name)
    }

    pub fn regulation_m_b_names(&self) -> impl Iterator<Item = &'static str> {
        REGULATION_M_B_POKEMON.iter().copied()
    }

    pub fn regulation_m_c_names(&self) -> impl Iterator<Item = &'static str> {
        REGULATION_M_C_POKEMON.iter().copied()
    }

    pub fn item_names(&self) -> impl Iterator<Item = &'static str> {
        POKEMON_CHAMPIONS_ITEMS.iter().copied()
    }

    pub fn ability_names(&self) -> impl Iterator<Item = &'static str> {
        CHAMPIONS_ABILITIES.iter().map(|ability| ability.name)
    }

    pub fn move_names(&self) -> impl Iterator<Item = &str> {
        self.moves_by_key.values().map(|move_| move_.name.as_str())
    }
}

impl SpeciesData {
    pub fn base_stats(&self) -> BaseStats {
        BaseStats::new(
            self.base_stats.hp,
            self.base_stats.attack,
            self.base_stats.defense,
            self.base_stats.special_attack,
            self.base_stats.special_defense,
            self.base_stats.speed,
        )
    }

    pub fn damage_types(&self) -> Result<[Option<PokemonType>; 2], DataError> {
        let first = self
            .types
            .first()
            .map(|value| parse_type(value))
            .transpose()?;
        let second = self
            .types
            .get(1)
            .map(|value| parse_type(value))
            .transpose()?;
        Ok([first, second])
    }
}

impl MoveData {
    pub fn to_damage_move(&self) -> Result<Move, DataError> {
        let mut move_ = Move::new(
            self.name.clone(),
            self.power,
            parse_type(&self.type_name)?,
            parse_category(&self.category)?,
        );
        move_.makes_contact = self.makes_contact;
        move_.is_priority = self.priority > 0;
        Ok(move_)
    }
}

pub fn parse_type(raw: &str) -> Result<PokemonType, DataError> {
    Ok(match raw.to_ascii_lowercase().as_str() {
        "normal" => PokemonType::Normal,
        "fire" => PokemonType::Fire,
        "water" => PokemonType::Water,
        "electric" => PokemonType::Electric,
        "grass" => PokemonType::Grass,
        "ice" => PokemonType::Ice,
        "fighting" => PokemonType::Fighting,
        "poison" => PokemonType::Poison,
        "ground" => PokemonType::Ground,
        "flying" => PokemonType::Flying,
        "psychic" => PokemonType::Psychic,
        "bug" => PokemonType::Bug,
        "rock" => PokemonType::Rock,
        "ghost" => PokemonType::Ghost,
        "dragon" => PokemonType::Dragon,
        "dark" => PokemonType::Dark,
        "steel" => PokemonType::Steel,
        "fairy" => PokemonType::Fairy,
        "stellar" => PokemonType::Stellar,
        "typeless" => PokemonType::Typeless,
        _ => return Err(DataError::UnknownType(raw.to_owned())),
    })
}

pub fn parse_category(raw: &str) -> Result<Category, DataError> {
    Ok(match raw.to_ascii_lowercase().as_str() {
        "physical" => Category::Physical,
        "special" => Category::Special,
        "status" => Category::Status,
        _ => return Err(DataError::UnknownCategory(raw.to_owned())),
    })
}

pub fn parse_ability(raw: &str) -> Result<Ability, DataError> {
    Ok(match normalize_name(raw).as_str() {
        "" | "none" | "nothing" => Ability::None,
        "adaptability" => Ability::Adaptability,
        "aerilate" => Ability::Aerilate,
        "airlock" => Ability::AirLock,
        "analytic" => Ability::Analytic,
        "armortail" => Ability::ArmorTail,
        "asone" => Ability::AsOne,
        "battery" => Ability::Battery,
        "battlebond" => Ability::BattleBond,
        "beadsofruin" => Ability::BeadsOfRuin,
        "blaze" => Ability::Blaze,
        "bulletproof" => Ability::Bulletproof,
        "clearbody" => Ability::ClearBody,
        "cloudnine" => Ability::CloudNine,
        "comatose" => Ability::Comatose,
        "competitive" => Ability::Competitive,
        "contrary" => Ability::Contrary,
        "dazzling" => Ability::Dazzling,
        "dauntlessshield" => Ability::DauntlessShield,
        "damp" => Ability::Damp,
        "defeatist" => Ability::Defeatist,
        "defiant" => Ability::Defiant,
        "disguise" => Ability::Disguise,
        "download" => Ability::Download,
        "dragonmaw" => Ability::DragonMaw,
        "dragonize" => Ability::Dragonize,
        "dryskin" => Ability::DrySkin,
        "eartheater" => Ability::EarthEater,
        "eelevate" => Ability::Eelevate,
        "effectspore" => Ability::EffectSpore,
        "electricsurge" => Ability::ElectricSurge,
        "electromorphosis" => Ability::Electromorphosis,
        "embodyaspect" => Ability::EmbodyAspect,
        "filter" => Ability::Filter,
        "fairyaura" => Ability::FairyAura,
        "firemane" => Ability::FireMane,
        "flareboost" => Ability::FlareBoost,
        "flashfire" => Ability::FlashFire,
        "flowergift" => Ability::FlowerGift,
        "flowerveil" => Ability::FlowerVeil,
        "fluffy" => Ability::Fluffy,
        "forecast" => Ability::Forecast,
        "forewarn" => Ability::Forewarn,
        "friendguard" => Ability::FriendGuard,
        "furcoat" => Ability::FurCoat,
        "fullmetalbody" => Ability::FullMetalBody,
        "galvanize" => Ability::Galvanize,
        "goodasgold" => Ability::GoodAsGold,
        "gooey" => Ability::Gooey,
        "grasspelt" => Ability::GrassPelt,
        "guarddog" => Ability::GuardDog,
        "guts" => Ability::Guts,
        "hadronengine" => Ability::HadronEngine,
        "heavymetal" => Ability::HeavyMetal,
        "heatproof" => Ability::Heatproof,
        "hypercutter" => Ability::HyperCutter,
        "hugepower" => Ability::HugePower,
        "hustle" => Ability::Hustle,
        "icescales" => Ability::IceScales,
        "infiltrator" => Ability::Infiltrator,
        "innerfocus" => Ability::InnerFocus,
        "intimidate" => Ability::Intimidate,
        "intrepidsword" => Ability::IntrepidSword,
        "unburden" => Ability::Unburden,
        "ironfist" => Ability::IronFist,
        "klutz" => Ability::Klutz,
        "leafguard" => Ability::LeafGuard,
        "levitate" => Ability::Levitate,
        "lightningrod" => Ability::LightningRod,
        "libero" => Ability::Libero,
        "lightmetal" => Ability::LightMetal,
        "liquidvoice" => Ability::LiquidVoice,
        "longreach" => Ability::LongReach,
        "megalauncher" => Ability::MegaLauncher,
        "megasol" => Ability::MegaSol,
        "marvelscale" => Ability::MarvelScale,
        "magicguard" => Ability::MagicGuard,
        "mimicry" => Ability::Mimicry,
        "mindeye" | "mindseye" => Ability::MindEye,
        "mirrorarmor" => Ability::MirrorArmor,
        "moldbreaker" => Ability::MoldBreaker,
        "motordrive" => Ability::MotorDrive,
        "multiscale" => Ability::Multiscale,
        "neuroforce" => Ability::Neuroforce,
        "neutralizinggas" => Ability::NeutralizingGas,
        "normalize" => Ability::Normalize,
        "oblivious" => Ability::Oblivious,
        "orichalcumpulse" => Ability::OrichalcumPulse,
        "owntempo" => Ability::OwnTempo,
        "overgrow" => Ability::Overgrow,
        "parentalbond" => Ability::ParentalBond,
        "pixilate" => Ability::Pixilate,
        "piercingdrill" => Ability::PiercingDrill,
        "plus" => Ability::Plus,
        "powerspot" => Ability::PowerSpot,
        "prismarmor" => Ability::PrismArmor,
        "protosynthesis" => Ability::Protosynthesis,
        "protean" => Ability::Protean,
        "punkrock" => Ability::PunkRock,
        "purepower" => Ability::PurePower,
        "purifyingsalt" => Ability::PurifyingSalt,
        "quarkdrive" => Ability::QuarkDrive,
        "queenlymajesty" => Ability::QueenlyMajesty,
        "rattled" => Ability::Rattled,
        "reckless" => Ability::Reckless,
        "refrigerate" => Ability::Refrigerate,
        "ripen" => Ability::Ripen,
        "rivalry" => Ability::Rivalry,
        "rockypayload" => Ability::RockyPayload,
        "sandforce" => Ability::SandForce,
        "sandspit" => Ability::SandSpit,
        "sapsipper" => Ability::SapSipper,
        "scrappy" => Ability::Scrappy,
        "sharpness" => Ability::Sharpness,
        "shadowshield" => Ability::ShadowShield,
        "skilllink" => Ability::SkillLink,
        "sheerforce" => Ability::SheerForce,
        "simple" => Ability::Simple,
        "sniper" => Ability::Sniper,
        "solidrock" => Ability::SolidRock,
        "solarpower" => Ability::SolarPower,
        "soundproof" => Ability::Soundproof,
        "spicyspray" => Ability::SpicySpray,
        "stamina" => Ability::Stamina,
        "stakeout" => Ability::Stakeout,
        "stormdrain" => Ability::StormDrain,
        "sturdy" => Ability::Sturdy,
        "steelworker" => Ability::Steelworker,
        "steelyspirit" => Ability::SteelySpirit,
        "strongjaw" => Ability::StrongJaw,
        "supersweetsyrup" => Ability::SupersweetSyrup,
        "supremeoverlord" => Ability::SupremeOverlord,
        "swarm" => Ability::Swarm,
        "swiftswim" => Ability::SwiftSwim,
        "swordofruin" => Ability::SwordOfRuin,
        "tabletsofruin" => Ability::TabletsOfRuin,
        "technician" => Ability::Technician,
        "thickfat" => Ability::ThickFat,
        "thermalexchange" => Ability::ThermalExchange,
        "tintedlens" => Ability::TintedLens,
        "tanglinghair" => Ability::TanglingHair,
        "torrent" => Ability::Torrent,
        "toughclaws" => Ability::ToughClaws,
        "cottondown" => Ability::CottonDown,
        "terashell" => Ability::TeraShell,
        "trace" => Ability::Trace,
        "transistor" => Ability::Transistor,
        "unaware" => Ability::Unaware,
        "unnerve" => Ability::Unnerve,
        "unseenfist" => Ability::UnseenFist,
        "vesselofruin" => Ability::VesselOfRuin,
        "voltabsorb" => Ability::VoltAbsorb,
        "weakarmor" => Ability::WeakArmor,
        "waterbubble" => Ability::WaterBubble,
        "waterabsorb" => Ability::WaterAbsorb,
        "waterveil" => Ability::WaterVeil,
        "minus" => Ability::Minus,
        "windpower" => Ability::WindPower,
        "windrider" => Ability::WindRider,
        "wonderguard" => Ability::WonderGuard,
        "whitesmoke" => Ability::WhiteSmoke,
        "aromaveil" => Ability::AromaVeil,
        "battlearmor" => Ability::BattleArmor,
        "chlorophyll" => Ability::Chlorophyll,
        "drought" => Ability::Drought,
        "drizzle" => Ability::Drizzle,
        "merciless" => Ability::Merciless,
        "quickfeet" => Ability::QuickFeet,
        "sandrush" => Ability::SandRush,
        "sandstream" => Ability::SandStream,
        "screencleaner" => Ability::ScreenCleaner,
        "shellarmor" => Ability::ShellArmor,
        "slushrush" => Ability::SlushRush,
        "snowwarning" => Ability::SnowWarning,
        "stalwart" => Ability::Stalwart,
        "surgesurfer" => Ability::SurgeSurfer,
        "stench" => Ability::Stench,
        "speedboost" => Ability::SpeedBoost,
        "limber" => Ability::Limber,
        "sandveil" => Ability::SandVeil,
        "static" => Ability::Static,
        "compoundeyes" => Ability::CompoundEyes,
        "insomnia" => Ability::Insomnia,
        "immunity" => Ability::Immunity,
        "shielddust" => Ability::ShieldDust,
        "suctioncups" => Ability::SuctionCups,
        "shadowtag" => Ability::ShadowTag,
        "roughskin" => Ability::RoughSkin,
        "synchronize" => Ability::Synchronize,
        "naturalcure" => Ability::NaturalCure,
        "illuminate" => Ability::Illuminate,
        "poisonpoint" => Ability::PoisonPoint,
        "magmaarmor" => Ability::MagmaArmor,
        "raindish" => Ability::RainDish,
        "pressure" => Ability::Pressure,
        "earlybird" => Ability::EarlyBird,
        "flamebody" => Ability::FlameBody,
        "keeneye" => Ability::KeenEye,
        "pickup" => Ability::Pickup,
        "cutecharm" => Ability::CuteCharm,
        "stickyhold" => Ability::StickyHold,
        "shedskin" => Ability::ShedSkin,
        "rockhead" => Ability::RockHead,
        "vitalspirit" => Ability::VitalSpirit,
        "tangledfeet" => Ability::TangledFeet,
        "steadfast" => Ability::Steadfast,
        "snowcloak" => Ability::SnowCloak,
        "gluttony" => Ability::Gluttony,
        "angerpoint" => Ability::AngerPoint,
        "poisonheal" => Ability::PoisonHeal,
        "hydration" => Ability::Hydration,
        "noguard" => Ability::NoGuard,
        "stall" => Ability::Stall,
        "superluck" => Ability::SuperLuck,
        "aftermath" => Ability::Aftermath,
        "anticipation" => Ability::Anticipation,
        "icebody" => Ability::IceBody,
        "frisk" => Ability::Frisk,
        "pickpocket" => Ability::Pickpocket,
        "cursedbody" => Ability::CursedBody,
        "healer" => Ability::Healer,
        "harvest" => Ability::Harvest,
        "telepathy" => Ability::Telepathy,
        "moody" => Ability::Moody,
        "overcoat" => Ability::Overcoat,
        "poisontouch" => Ability::PoisonTouch,
        "regenerator" => Ability::Regenerator,
        "bigpecks" => Ability::BigPecks,
        "illusion" => Ability::Illusion,
        "imposter" => Ability::Imposter,
        "mummy" => Ability::Mummy,
        "moxie" => Ability::Moxie,
        "justified" => Ability::Justified,
        "magicbounce" => Ability::MagicBounce,
        "prankster" => Ability::Prankster,
        "cheekpouch" => Ability::CheekPouch,
        "magician" => Ability::Magician,
        "sweetveil" => Ability::SweetVeil,
        "stancechange" => Ability::StanceChange,
        "galewings" => Ability::GaleWings,
        "symbiosis" => Ability::Symbiosis,
        "berserk" => Ability::Berserk,
        "corrosion" => Ability::Corrosion,
        "innardsout" => Ability::InnardsOut,
        "receiver" => Ability::Receiver,
        "wanderingspirit" => Ability::WanderingSpirit,
        "hungerswitch" => Ability::HungerSwitch,
        "quickdraw" => Ability::QuickDraw,
        "curiousmedicine" => Ability::CuriousMedicine,
        "zerotohero" => Ability::ZeroToHero,
        "opportunist" => Ability::Opportunist,
        "cudchew" => Ability::CudChew,
        "toxicdebris" => Ability::ToxicDebris,
        "hospitality" => Ability::Hospitality,
        "auraguard" => Ability::AuraGuard,
        // Terrain setters are represented by explicit Field terrain in the damage library.
        "psychicsurge" | "grassysurge" => Ability::None,
        "liquidooze" => Ability::LiquidOoze,
        "runaway" => Ability::RunAway,
        "emergencyexit" => Ability::EmergencyExit,
        "seedsower" => Ability::SeedSower,
        _ => return Err(DataError::UnknownAbility(raw.to_owned())),
    })
}

pub fn parse_item(raw: &str) -> Result<Item, DataError> {
    let key = normalize_name(raw);
    if let Some((_, item)) = CHAMPIONS_ITEM_VALUES
        .iter()
        .find(|(name, _)| normalize_name(name) == key)
    {
        return Ok(*item);
    }
    Ok(match key.as_str() {
        "" | "none" | "nothing" => Item::None,
        "abilityshield" => Item::AbilityShield,
        "adrenalineorb" => Item::AdrenalineOrb,
        "assaultvest" => Item::AssaultVest,
        "airballoon" => Item::AirBalloon,
        "choiceband" => Item::ChoiceBand,
        "choicescarf" => Item::ChoiceScarf,
        "choicespecs" => Item::ChoiceSpecs,
        "clearamulet" => Item::ClearAmulet,
        "boosterenergy" => Item::BoosterEnergy,
        "cornerstonemask" => Item::CornerstoneMask,
        "expertbelt" => Item::ExpertBelt,
        "electricseed" => Item::ElectricSeed,
        "eviolite" => Item::Eviolite,
        "floatstone" => Item::FloatStone,
        "focussash" => Item::FocusSash,
        "grassyseed" => Item::GrassySeed,
        "hearthflamemask" => Item::HearthflameMask,
        "ironball" => Item::IronBall,
        "lightball" => Item::LightBall,
        "leftovers" => Item::Leftovers,
        "lifeorb" => Item::LifeOrb,
        "mentalherb" => Item::MentalHerb,
        "mistyseed" => Item::MistySeed,
        "muscleband" => Item::MuscleBand,
        "wiseglasses" => Item::WiseGlasses,
        "punchingglove" => Item::PunchingGlove,
        "protectivepads" => Item::ProtectivePads,
        "psychicseed" => Item::PsychicSeed,
        "ringtarget" => Item::RingTarget,
        "scopelens" => Item::ScopeLens,
        "shellbell" => Item::ShellBell,
        "utilityumbrella" => Item::UtilityUmbrella,
        "wellspringmask" => Item::WellspringMask,
        "flameplate" => Item::FlamePlate,
        "splashplate" => Item::SplashPlate,
        "zapplate" => Item::ZapPlate,
        "meadowplate" => Item::MeadowPlate,
        "icicleplate" => Item::IciclePlate,
        "fistplate" => Item::FistPlate,
        "toxicplate" => Item::ToxicPlate,
        "earthplate" => Item::EarthPlate,
        "skyplate" => Item::SkyPlate,
        "mindplate" => Item::MindPlate,
        "insectplate" => Item::InsectPlate,
        "stoneplate" => Item::StonePlate,
        "spookyplate" => Item::SpookyPlate,
        "dracoplate" => Item::DracoPlate,
        "dreadplate" => Item::DreadPlate,
        "ironplate" => Item::IronPlate,
        "silkscarf" => Item::SilkScarf,
        "blackbelt" => Item::BlackBelt,
        "blackglasses" => Item::BlackGlasses,
        "charcoal" => Item::Charcoal,
        "dragonfang" => Item::DragonFang,
        "hardstone" => Item::HardStone,
        "magnet" => Item::Magnet,
        "metalcoat" => Item::MetalCoat,
        "miracleseed" => Item::MiracleSeed,
        "mysticwater" => Item::MysticWater,
        "nevermeltice" => Item::NeverMeltIce,
        "poisonbarb" => Item::PoisonBarb,
        "sharpbeak" => Item::SharpBeak,
        "silverpowder" => Item::SilverPowder,
        "softsand" => Item::SoftSand,
        "spelltag" => Item::SpellTag,
        "twistedspoon" => Item::TwistedSpoon,
        "fairyfeather" => Item::FairyFeather,
        "abomasite" => Item::None,
        "aggronite" => Item::None,
        "barbaracleite" => Item::None,
        "beedrillite" => Item::None,
        "bigroot" => Item::None,
        "brightpowder" => Item::None,
        "venusaurite" => Item::Venusaurite,
        "charizarditex" => Item::CharizarditeX,
        "charizarditey" => Item::CharizarditeY,
        "blastoisinite" => Item::Blastoisinite,
        "blazikenite" => Item::None,
        "pidgeotite" => Item::Pidgeotite,
        "clefablite" => Item::Clefablite,
        "alakazite" => Item::Alakazite,
        "victreebelite" => Item::Victreebelite,
        "slowbronite" => Item::Slowbronite,
        "gengarite" => Item::Gengarite,
        "kangaskhanite" => Item::Kangaskhanite,
        "starminite" => Item::Starminite,
        "pinsirite" => Item::Pinsirite,
        "aerodactylite" => Item::Aerodactylite,
        "dragoninite" => Item::Dragoninite,
        "meganiumite" => Item::Meganiumite,
        "feraligite" => Item::Feraligite,
        "ampharosite" => Item::Ampharosite,
        "scizorite" => Item::Scizorite,
        "skarmorite" => Item::Skarmorite,
        "houndoominite" => Item::Houndoominite,
        "tyranitarite" => Item::Tyranitarite,
        "gardevoirite" => Item::Gardevoirite,
        "sablenite" => Item::Sablenite,
        "medichamite" => Item::Medichamite,
        "sharpedonite" => Item::Sharpedonite,
        "cameruptite" => Item::Cameruptite,
        "altarianite" => Item::Altarianite,
        "banettite" => Item::Banettite,
        "chimechite" => Item::Chimechite,
        "absolite" => Item::Absolite,
        "glalitite" => Item::Glalitite,
        "lopunnite" => Item::Lopunnite,
        "lucarionite" => Item::Lucarionite,
        "galladite" => Item::Galladite,
        "froslassite" => Item::Froslassite,
        "emboarite" => Item::Emboarite,
        "excadrite" => Item::Excadrite,
        "audinite" => Item::Audinite,
        "chesnaughtite" => Item::None,
        "chandelurite" => Item::Chandelurite,
        "damprock" => Item::None,
        "delphoxite" => Item::None,
        "dragalgeite" => Item::None,
        "eelektrossite" => Item::None,
        "falinksite" => Item::None,
        "floettite" => Item::None,
        "focusband" => Item::None,
        "garchompite" => Item::None,
        "golurkite" => Item::Golurkite,
        "greninjite" => Item::None,
        "gyaradosite" => Item::None,
        "meowsticite" => Item::Meowsticite,
        "hawluchanite" => Item::Hawluchanite,
        "heatrock" => Item::None,
        "heracronite" => Item::None,
        "icyrock" => Item::None,
        "kingsrock" => Item::None,
        "lightclay" => Item::None,
        "malamarite" => Item::None,
        "manectite" => Item::None,
        "mawileite" => Item::None,
        "metagrossite" => Item::None,
        "metronome" => Item::None,
        "crabominite" => Item::Crabominite,
        "pyroarite" => Item::None,
        "quickclaw" => Item::None,
        "raichunitex" => Item::None,
        "raichunitey" => Item::None,
        "sceptileite" => Item::None,
        "scolipedeite" => Item::None,
        "scraftyite" => Item::None,
        "shedshell" => Item::None,
        "smoothrock" => Item::None,
        "staraptorite" => Item::None,
        "steelixite" => Item::None,
        "swampertite" => Item::None,
        "drampanite" => Item::Drampanite,
        "scovillainite" => Item::Scovillainite,
        "glimmoranite" => Item::Glimmoranite,
        "widelens" => Item::None,
        "zoomlens" => Item::None,
        "burndrive" => Item::BurnDrive,
        "chilldrive" => Item::ChillDrive,
        "dousedrive" => Item::DouseDrive,
        "shockdrive" => Item::ShockDrive,
        "bugmemory" => Item::BugMemory,
        "darkmemory" => Item::DarkMemory,
        "dragonmemory" => Item::DragonMemory,
        "electricmemory" => Item::ElectricMemory,
        "fairymemory" => Item::FairyMemory,
        "fightingmemory" => Item::FightingMemory,
        "firememory" => Item::FireMemory,
        "flyingmemory" => Item::FlyingMemory,
        "whiteherb" => Item::None,
        "ghostmemory" => Item::GhostMemory,
        "grassmemory" => Item::GrassMemory,
        "groundmemory" => Item::GroundMemory,
        "icememory" => Item::IceMemory,
        "poisonmemory" => Item::PoisonMemory,
        "psychicmemory" => Item::PsychicMemory,
        "rockmemory" => Item::RockMemory,
        "steelmemory" => Item::SteelMemory,
        "watermemory" => Item::WaterMemory,
        "aguavberry" => Item::AguavBerry,
        "apicotberry" => Item::ApicotBerry,
        "aspearberry" => Item::AspearBerry,
        "belueberry" => Item::BelueBerry,
        "blukberry" => Item::BlukBerry,
        "chilanberry" => Item::ChilanBerry,
        "cheriberry" => Item::CheriBerry,
        "chestoberry" => Item::ChestoBerry,
        "occaberry" => Item::OccaBerry,
        "passhoberry" => Item::PasshoBerry,
        "wacanberry" => Item::WacanBerry,
        "rindoberry" => Item::RindoBerry,
        "yacheberry" => Item::YacheBerry,
        "chopleberry" => Item::ChopleBerry,
        "kebiaberry" => Item::KebiaBerry,
        "shucaberry" => Item::ShucaBerry,
        "cobaberry" => Item::CobaBerry,
        "payapaberry" => Item::PayapaBerry,
        "tangaberry" => Item::TangaBerry,
        "chartiberry" => Item::ChartiBerry,
        "kasibberry" => Item::KasibBerry,
        "habanberry" => Item::HabanBerry,
        "colburberry" => Item::ColburBerry,
        "bibiriberry" | "babiriberry" => Item::BabiriBerry,
        "roseliberry" => Item::RoseliBerry,
        "cornnberry" => Item::CornnBerry,
        "custapberry" => Item::CustapBerry,
        "durinberry" => Item::DurinBerry,
        "enigmaberry" => Item::EnigmaBerry,
        "figyberry" => Item::FigyBerry,
        "ganlonberry" => Item::GanlonBerry,
        "grepaberry" => Item::GrepaBerry,
        "hondewberry" => Item::HondewBerry,
        "iapapaberry" => Item::IapapaBerry,
        "jabocaberry" => Item::JabocaBerry,
        "keeberry" => Item::KeeBerry,
        "lansatberry" => Item::LansatBerry,
        "leppaberry" => Item::LeppaBerry,
        "liechiberry" => Item::LiechiBerry,
        "lumberry" => Item::LumBerry,
        "magoberry" => Item::MagoBerry,
        "magostberry" => Item::MagostBerry,
        "marangaberry" => Item::MarangaBerry,
        "micleberry" => Item::MicleBerry,
        "nanabberry" => Item::NanabBerry,
        "nomelberry" => Item::NomelBerry,
        "oranberry" => Item::OranBerry,
        "pamtreberry" => Item::PamtreBerry,
        "pechaberry" => Item::PechaBerry,
        "persimberry" => Item::PersimBerry,
        "petayaberry" => Item::PetayaBerry,
        "pinapberry" => Item::PinapBerry,
        "pomegberry" => Item::PomegBerry,
        "qualotberry" => Item::QualotBerry,
        "rabutaberry" => Item::RabutaBerry,
        "rawstberry" => Item::RawstBerry,
        "razzberry" => Item::RazzBerry,
        "rowapberry" => Item::RowapBerry,
        "salacberry" => Item::SalacBerry,
        "sitrusberry" => Item::SitrusBerry,
        "spelonberry" => Item::SpelonBerry,
        "starfberry" => Item::StarfBerry,
        "tamatoberry" => Item::TamatoBerry,
        "watmelberry" => Item::WatmelBerry,
        "wepearberry" => Item::WepearBerry,
        "wikiberry" => Item::WikiBerry,
        "normalgem" => Item::NormalGem,
        "firegem" => Item::FireGem,
        "watergem" => Item::WaterGem,
        "electricgem" => Item::ElectricGem,
        "grassgem" => Item::GrassGem,
        "icegem" => Item::IceGem,
        "fightinggem" => Item::FightingGem,
        "poisongem" => Item::PoisonGem,
        "groundgem" => Item::GroundGem,
        "flyinggem" => Item::FlyingGem,
        "psychicgem" => Item::PsychicGem,
        "buggem" => Item::BugGem,
        "rockgem" => Item::RockGem,
        "ghostgem" => Item::GhostGem,
        "dragongem" => Item::DragonGem,
        "darkgem" => Item::DarkGem,
        "steelgem" => Item::SteelGem,
        "fairygem" => Item::FairyGem,
        _ => return Err(DataError::UnknownItem(raw.to_owned())),
    })
}

pub fn parse_spread_key(
    raw: &str,
) -> Result<(damage_calc::Nature, crate::stats::StatPoints), DataError> {
    let (nature, spread) = raw
        .split_once(':')
        .ok_or_else(|| DataError::MalformedSpread(raw.to_owned()))?;
    let nature =
        parse_nature_name(nature).ok_or_else(|| DataError::UnknownNature(nature.to_owned()))?;
    let values = spread
        .split('/')
        .map(str::parse::<u16>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| DataError::MalformedSpread(raw.to_owned()))?;
    if values.len() != 6 {
        return Err(DataError::MalformedSpread(raw.to_owned()));
    }
    Ok((
        nature,
        crate::stats::StatPoints::new(
            values[0], values[1], values[2], values[3], values[4], values[5],
        ),
    ))
}

pub fn normalize_name(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_champions_data() {
        let data = ChampionsData::load().unwrap();
        let venusaur = data.species("Venusaur").unwrap();
        assert_eq!(venusaur.base_stats.hp, 80);
        assert_eq!(
            data.species("Charizard-Mega-Y").unwrap().display_name,
            "Mega Charizard Y"
        );
        let flamethrower = data.move_data("Flamethrower").unwrap();
        assert_eq!(flamethrower.type_name, "Fire");
    }

    #[test]
    fn exposes_regulation_m_b_roster() {
        let data = ChampionsData::load().unwrap();
        assert_eq!(data.regulation_m_b_names().count(), 247);
        assert!(data
            .regulation_m_b_names()
            .any(|name| name == "Mega Pyroar"));
    }

    #[test]
    fn exposes_and_resolves_regulation_m_c_additions() {
        let data = ChampionsData::load().unwrap();
        assert_eq!(data.regulation_m_c_names().count(), 279);
        for name in [
            "Wigglytuff",
            "Persian",
            "Persian (Alolan)",
            "Farfetch’d",
            "Mr. Mime",
            "Swalot",
            "Salamence",
            "Gogoat",
            "Golisopod",
            "Rillaboom",
            "Cinderace",
            "Inteleon",
            "Thievul",
            "Toxtricity (Amped Form)",
            "Toxtricity (Low Key Form)",
            "Grapploct",
            "Perrserker",
            "Sirfetch’d",
            "Pincurchin",
            "Indeedee (Male)",
            "Indeedee (Female)",
            "Pawmot",
            "Arboliva",
            "Mabosstiff",
            "Baxcalibur",
            "Squawkabilly",
            "Mega Absol Z",
            "Mega Salamence",
            "Mega Garchomp Z",
            "Mega Lucario Z",
            "Mega Golisopod",
            "Mega Baxcalibur",
        ] {
            assert!(
                data.regulation_m_c_names().any(|entry| entry == name),
                "{name}"
            );
            data.species(name)
                .unwrap_or_else(|err| panic!("{name}: {err}"));
        }
        for (alias, canonical) in [
            ("Persian-Alola", "Persian (Alolan)"),
            ("Toxtricity", "Toxtricity (Amped Form)"),
            ("Toxtricity-Amped", "Toxtricity (Amped Form)"),
            ("Toxtricity-Low-Key", "Toxtricity (Low Key Form)"),
            ("Indeedee", "Indeedee (Male)"),
            ("Indeedee-M", "Indeedee (Male)"),
            ("Indeedee-F", "Indeedee (Female)"),
            ("Squawkabilly", "Squawkabilly (Green Plumage)"),
            ("Squawkabilly-Green", "Squawkabilly (Green Plumage)"),
            ("Squawkabilly-Blue", "Squawkabilly (Blue Plumage)"),
            ("Squawkabilly-Yellow", "Squawkabilly (Yellow Plumage)"),
            ("Squawkabilly-White", "Squawkabilly (White Plumage)"),
            ("Absol-Mega-Z", "Mega Absol Z"),
            ("Salamence-Mega", "Mega Salamence"),
            ("Garchomp-Mega-Z", "Mega Garchomp Z"),
            ("Lucario-Mega-Z", "Mega Lucario Z"),
            ("Golisopod-Mega", "Mega Golisopod"),
            ("Baxcalibur-Mega", "Mega Baxcalibur"),
        ] {
            assert_eq!(data.species(alias).unwrap().display_name, canonical);
        }
        let lucario = data.species("Lucario-Mega-Z").unwrap();
        assert_eq!(lucario.base_stats.special_attack, 164);
        assert_eq!(lucario.base_stats.speed, 151);
        assert_eq!(parse_ability("Aura Guard").unwrap(), Ability::AuraGuard);
    }

    #[test]
    fn resolves_all_current_abilities_and_items() {
        let data = ChampionsData::load().unwrap();
        for name in data.ability_names() {
            parse_ability(name).unwrap_or_else(|err| panic!("{name}: {err}"));
        }
        for (name, expected) in CHAMPIONS_ITEM_VALUES {
            assert_eq!(parse_item(name).unwrap(), *expected, "{name}");
        }
        for (name, expected) in [
            ("Leek", Item::Leek),
            ("Rocky Helmet", Item::RockyHelmet),
            ("Air Balloon", Item::AirBalloon),
            ("Red Card", Item::RedCard),
            ("Binding Band", Item::BindingBand),
            ("Eject Button", Item::EjectButton),
            ("Normal Gem", Item::NormalGem),
            ("Terrain Extender", Item::TerrainExtender),
            ("Electric Seed", Item::ElectricSeed),
            ("Psychic Seed", Item::PsychicSeed),
            ("Misty Seed", Item::MistySeed),
            ("Grassy Seed", Item::GrassySeed),
        ] {
            assert!(data.item_names().any(|entry| entry == name), "{name}");
            assert_eq!(parse_item(name).unwrap(), expected);
        }
        assert!(parse_ability("Not An Ability").is_err());
        assert!(parse_item("Not An Item").is_err());
    }

    #[test]
    fn parses_spread_key() {
        let (nature, points) = parse_spread_key("Jolly:0/32/2/0/0/32").unwrap();
        assert_eq!(nature, damage_calc::Nature::Jolly);
        assert_eq!(points.attack, 32);
        assert_eq!(points.defense, 2);
    }

    #[test]
    fn parses_normalized_ability_and_item_names() {
        assert_eq!(parse_ability("Solar Power").unwrap(), Ability::SolarPower);
        assert_eq!(parse_ability("solarpower").unwrap(), Ability::SolarPower);
        assert_eq!(parse_ability("Fairy Aura").unwrap(), Ability::FairyAura);
        assert_eq!(parse_ability("Flower Veil").unwrap(), Ability::FlowerVeil);
        assert_eq!(parse_ability("Unburden").unwrap(), Ability::Unburden);
        assert_eq!(parse_ability("Rivalry").unwrap(), Ability::Rivalry);
        assert_eq!(parse_ability("Skill Link").unwrap(), Ability::SkillLink);
        assert_eq!(parse_ability("Swift Swim").unwrap(), Ability::SwiftSwim);
        assert_eq!(parse_ability("Eelevate").unwrap(), Ability::Eelevate);
        assert_eq!(parse_ability("Effect Spore").unwrap(), Ability::EffectSpore);
        assert_eq!(
            parse_ability("Electric Surge").unwrap(),
            Ability::ElectricSurge
        );
        assert_eq!(parse_ability("Fire Mane").unwrap(), Ability::FireMane);
        assert_eq!(parse_ability("Forewarn").unwrap(), Ability::Forewarn);
        assert_eq!(parse_ability("Good as Gold").unwrap(), Ability::GoodAsGold);
        assert_eq!(parse_item("Choice Scarf").unwrap(), Item::ChoiceScarf);
        assert_eq!(parse_item("choicescarf").unwrap(), Item::ChoiceScarf);
        assert_eq!(parse_item("Focus Sash").unwrap(), Item::FocusSash);
        assert_eq!(parse_item("Leftovers").unwrap(), Item::Leftovers);
        assert_eq!(parse_item("White Herb").unwrap(), Item::WhiteHerb);
        assert_eq!(parse_item("nothing").unwrap(), Item::None);
        for item in POKEMON_CHAMPIONS_ITEMS {
            parse_item(item).unwrap_or_else(|err| panic!("{item} failed to parse: {err}"));
        }
    }
}
