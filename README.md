# SpreadLab

SpreadLab is an alpha Pokemon Champions Stat Point optimizer for:

```text
[Gen 9 Champions] VGC 2026 Reg M-A (Bo3)
```

> Alpha status: interfaces, CLI output, WebUI controls, and optimizer reports may
> change while the damage library and Champions rules coverage are still moving.
> Use results as a practical helper, not as a final rules oracle.

Showdown / Smogon chaos format id:

```text
gen9championsvgc2026regmabo3
```

## Ground Truth

Damage calculations are delegated to:

```toml
damage_calc = { package = "pkmn-dmg-lib", git = "https://github.com/D35P4C1T0/pkmn-dmg-lib-rs.git", features = ["serde"] }
```

This project generates legal Champions SP spreads, parses sets, fetches stats,
and builds damage inputs. It does not reimplement damage formulas.

## Features

- CLI for parsing Showdown sets, checking final Champions stats, running damage
  calcs, and searching offensive/defensive spreads.
- Optional local WebUI with attacker/defender set editors, field controls, and
  result tables.
- Public Rust API for external tools and visualizers.
- Smogon usage-data fetching/cache helpers for Champions formats.

## Quick Start

```sh
git clone https://github.com/D35P4C1T0/SpreadLab.git
cd SpreadLab
cargo test
cargo run -- --help
```

Run the local WebUI:

```sh
cargo run --features webui -- serve
```

Then open:

```text
http://127.0.0.1:8080
```

## Local Damage Library Development

For local work, add this to `Cargo.toml` temporarily:

```toml
[patch."https://github.com/D35P4C1T0/pkmn-dmg-lib-rs.git"]
pkmn-dmg-lib = { path = "../pkmn-dmg-lib-rs" }
```

## Commands

Fetch latest available monthly chaos data:

```sh
cargo run -- fetch --month latest --rating 1760
```

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

Print pinned Champions names from the damage library:

```sh
cargo run -- list species
cargo run -- list regulation
cargo run -- list items
cargo run -- list abilities
cargo run -- list moves
```

Start the local WebUI:

```sh
cargo run --features webui -- serve
```

Then open:

```text
http://127.0.0.1:8080
```

## WebUI Field Controls

The WebUI exposes the current battle-field switches that are available through
`pkmn-dmg-lib-rs`. Supported controls include:

- format: Singles, Doubles
- terrain: Electric, Grassy, Misty, Psychic
- weather: Sun, Rain, Sand, Snow
- Fairy Aura, Gravity, Helping Hand, Tailwind
- defender-side Protect, Aurora Veil, Reflect, Light Screen, Friend Guard

Controls that are visible but disabled are not supported by the current damage
library API yet. At the time of writing this includes Stealth Rock, Spikes, Salt
Cure, and several attacker-side defensive conditions.

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
      "move": "Flamethrower"
    }
  ]
}
```

## Status

Implemented first:

- Champions SP and EV parser
- `EVs:` to `SPs:` conversion with `floor((EV + 4) / 8)`
- canonical `SPs:` export
- stat conversion wrapper around `pkmn-dmg-lib-rs`
- Champions data resolver from `damage_calc::data::CHAMPIONS_DATA_JSON`
- pinned Champions species/item/ability lists from `pkmn-dmg-lib-rs`
- Smogon chaos fetch/cache/normalization
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
