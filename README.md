# SpreadLab

SpreadLab is an alpha Pokemon Champions Stat Point optimizer for:

```text
[Gen 9 Champions] VGC 2026 Reg M-C (Bo3)
```

> Alpha status: interfaces, CLI output, public API structs, and optimizer reports may
> change while the damage library and Champions rules coverage are still moving.
> Use results as a practical helper, not as a final rules oracle.

## Ground Truth

Damage calculations are delegated to:

```toml
damage_calc = { package = "pkmn-dmg-lib", git = "https://github.com/D35P4C1T0/pkmn-dmg-lib-rs.git", rev = "b8df8c49122f0f6016ecd049efca1824d59effe4", features = ["serde"] }
```

This project generates legal Champions SP spreads, parses sets, and builds
damage inputs. It does not reimplement damage formulas.

## Regulation M-C data and calculation scope

CLI `list regulation` and API/WASM metadata expose M-C: 279 roster entries,
including the 23 newly usable species (26 regular entries) and six Megas.
The normalized species data also includes all four Squawkabilly plumages.
The Rust API retains `regulation_m_b_names()` and adds `regulation_m_c_names()`.
Items and typed item identities now come directly from the pinned damage library,
including the 12 new held items, six Mega Stones, and corrected stone spellings.
Showdown aliases such as `Persian-Alola`, `Indeedee-F`, `Toxtricity-Low-Key`,
and `Lucario-Mega-Z` resolve to their source-data forms.

Sources checked on 2026-09-11:

- [Official M-C announcement](https://news.pokemon-home.com/en/page/816.html).
- [Project Pokemon champout](https://github.com/projectpokemon/champout) for
  species stats, types, weights, abilities, and moves. A fresh `personal.json`
  download matched the pinned library's source manifest SHA-256:
  `5ab5444e4e08cc692c1c309dc0c113e8ba63e23187f9f6331cc4505db3ab96a7`.
- [Damage library data provenance and scope](https://github.com/D35P4C1T0/pkmn-dmg-lib-rs/blob/b8df8c49122f0f6016ecd049efca1824d59effe4/data/champions/README.md).

All listed abilities parse, including Aura Guard, Emergency Exit, and Seed Sower.
Grassy Surge and Psychic Surge have no typed ability variant upstream; they parse
as `Ability::None`, with terrain supplied explicitly through `FieldRequest.terrain`
(`"Grassy"` / `"Psychic"`) or the Rust benchmark field. Terrain is not inferred
from set abilities. Supply the current battle state when using terrain Seeds.

Damage and KO estimates follow the pinned library: Parental Bond uses a
quarter-strength second hit, and KO projections do not model Focus Sash
activation. The library models Aura Guard contact reduction, Air Balloon
immunity, Normal Gem, and terrain Seeds. Leek's critical-hit chance, Rocky Helmet
retaliation, switching items, trapping duration, and terrain duration are outside
its single-attack calculation; critical hits and field state remain explicit inputs.

## Features

- CLI for parsing Showdown sets, checking final Champions stats, running damage
  calcs, and searching offensive/defensive spreads.
- Public Rust API for external tools and visualizers.
- CLI/library-only crate. The embedded alpha WebUI was removed; see
  `handout.md` for the handoff notes for a future separate WebUI.

## Quick Start

```sh
git clone https://github.com/D35P4C1T0/SpreadLab.git
cd SpreadLab
cargo test
cargo run -- --help
```

## Quality Gate

Run this gate before committing changes:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## WASM

Build the Rust library for browser packaging:

```sh
rustup target add wasm32-unknown-unknown
cargo build --lib --target wasm32-unknown-unknown
```

The wasm module exports JSON-string functions for browser callers:

- `loadMetadata()`
- `calculateDamage(requestJson)`
- `findMinHpDefSurvival(requestJson)`
- `findMinCombinedHpDefSurvival(requestJson)`
- `findMinOffensiveKo(requestJson)`
- `runDefensiveOptimization(requestJson)`
- `runOffensiveOptimization(requestJson)`

Use `wasm-bindgen` or `wasm-pack` to generate JavaScript glue for the browser.

## Local Damage Library Development

To develop against a local damage library checkout, add this optional patch:

```toml
[patch."https://github.com/D35P4C1T0/pkmn-dmg-lib-rs.git"]
pkmn-dmg-lib = { path = "../pkmn-dmg-lib" }
```

## Commands

Parse a Showdown set:

```sh
cargo run -- parse set.txt
```

Print final raw Champions stats:

```sh
cargo run -- stats set.txt
```

Run one damage calculation:

```sh
cargo run -- calc --attacker attacker.txt --defender defender.txt --move "Flamethrower"
```

Force a critical hit for damage, survival, KO, or one-off optimization benchmarks:

```sh
cargo run -- calc --attacker attacker.txt --defender defender.txt --move "Flamethrower" --crit
cargo run -- survive --attacker attacker.txt --defender defender.txt --move "Iron Head" --crit
```

Print pinned Champions names from the damage library:

```sh
cargo run -- list species
cargo run -- list regulation
cargo run -- list items
cargo run -- list abilities
cargo run -- list moves
```

Search defensive spreads for one benchmark:

```sh
cargo run -- optimize defensive --attacker attacker.txt --defender defender.txt --move "Close Combat" --full-spend --lock-atk 0 --lock-spa 0 --lock-spe 0
```

Search offensive spreads for one benchmark:

```sh
cargo run -- optimize offensive --attacker attacker.txt --defender defender.txt --move "Flamethrower" --full-spend --lock-atk 0
```

Find minimum offensive investment for a guaranteed KO:

```sh
cargo run -- ko --attacker attacker.txt --defender defender.txt --move "Last Respects" --move-times-affected 1 --min-ko-chance 1.0
```

Only compare relevant boosting nature and neutral nature:

```sh
cargo run -- ko --attacker attacker.txt --defender defender.txt --move "Last Respects" --move-times-affected 1 --min-ko-chance 1.0 --optimize-nature
cargo run -- survive --attacker attacker.txt --defender defender.txt --move "Iron Head" --max-ko-chance 0.125 --optimize-nature
```

Start a defensive search from partial HP:

```sh
cargo run -- survive --attacker attacker.txt --defender defender.txt --move "Iron Head" --hp-percent 75 --max-ko-chance 0.125
```

Find a spread that survives two attacks in a row:

```sh
cargo run -- survive-sequence --attacker1 attacker-a.txt --move1 "Iron Head" --attacker2 attacker-b.txt --move2 "Rock Slide" --defender defender.txt --max-ko-chance 0.125 --hp-percent 100 --optimize-nature
```

Show closest failing spread when nothing satisfies the requested chance:

```sh
cargo run -- survive --attacker attacker.txt --defender defender.txt --move "Rock Slide" --max-ko-chance 0 --optimize-nature --show-closest-miss
cargo run -- ko --attacker attacker.txt --defender defender.txt --move "Last Respects" --min-ko-chance 1 --show-closest-miss
```

Search against multiple benchmarks:

```sh
cargo run -- optimize defensive --benchmarks benchmarks.json --full-spend --lock-atk 0 --lock-spa 0 --lock-spe 0
```

`benchmarks.json`:

```json
{
  "benchmarks": [
    {
      "attacker": "Charizard-Mega-Y @ Charizardite Y\nAbility: Solar Power\nSPs: 2 HP / 32 SpA / 32 Spe\nTimid Nature\n- Flamethrower",
      "defender": "Venusaur @ Sitrus Berry\nAbility: Overgrow\nSPs: 32 HP / 32 SpD / 2 Spe\nCalm Nature\n- Protect",
      "move": "Flamethrower",
      "critical": false
    }
  ]
}
```

## Status

Implemented first:

- Champions `SPs:` parser and canonical export
- Low-value `EVs:` parser for Champions point exports where all values are
  `<= 32`
- Legacy `EVs:` to `SPs:` conversion with `floor((EV + 4) / 8)` when any value
  is greater than `32`
- stat conversion wrapper around `pkmn-dmg-lib-rs`
- Champions data resolver from `damage_calc::data::CHAMPIONS_DATA_JSON`
- pinned Champions species/item/ability lists from `pkmn-dmg-lib-rs`
- legal SP spread generation
- single benchmark damage bridge
- basic ranked defensive/offensive search
- normalized item/ability resolver for damage-lib enum names
- JSON benchmark files for batch defensive/offensive searches
- public API methods for external visualizers:
  - `calculate_damage_request`
  - `calculate_damage_request_with_data`
  - `find_min_hp_def_survival`
  - `find_min_hp_def_survival_with_data`
  - `find_min_combined_hp_def_survival`
  - `find_min_combined_hp_def_survival_with_data`
  - `find_min_offensive_ko`
  - `find_min_offensive_ko_with_data`

## Library API Example

```rust
use spreadlab_rs::api::{
    find_min_hp_def_survival, HpDefSurvivalRequest,
};

let result = find_min_hp_def_survival(HpDefSurvivalRequest {
    attacker_set: "Kingambit\nAbility: Defiant\nSPs: 32 Atk\nAdamant Nature\n- Iron Head".into(),
    defender_set: "Mega Floette\n- Protect".into(),
    move_name: "Iron Head".into(),
    max_ko_chance: 0.125,
    hp_percent: None,
    nature: None,
    optimize_nature: true,
    limit: 10,
    move_times_affected: 0,
    field: None,
})?;

let best = result.best.expect("at least one survival spread");
assert_eq!(best.total_points, 24);
```

Still to build:

- richer item/ability resolver coverage
- report output for ranked results

## License

MIT. See [LICENSE](LICENSE).
