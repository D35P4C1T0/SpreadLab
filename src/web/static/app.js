const natures = [
  "Adamant", "Bashful", "Bold", "Brave", "Calm", "Careful", "Docile", "Gentle", "Hardy",
  "Hasty", "Impish", "Jolly", "Lax", "Lonely", "Mild", "Modest", "Naive", "Naughty",
  "Quiet", "Quirky", "Rash", "Relaxed", "Sassy", "Serious", "Timid",
];

const sampleAttacker = `Kingambit @ Black Glasses
Ability: Defiant
SPs: 32 Atk
Adamant Nature
- Iron Head
- Kowtow Cleave`;

const sampleDefender = `Mega Floette
SPs: 4 HP
Bold Nature
- Protect`;

const state = { mode: "damage" };
const $ = (id) => document.getElementById(id);

function init() {
  $("attacker-set").value = sampleAttacker;
  $("defender-set").value = sampleDefender;
  for (const nature of natures) {
    $("nature").append(new Option(nature, nature));
  }
  document.querySelectorAll(".tab").forEach((tab) => {
    tab.addEventListener("click", () => setMode(tab.dataset.mode));
  });
  $("run").addEventListener("click", run);
  $("swap").addEventListener("click", swapSets);
  $("clear").addEventListener("click", clearResults);
  $("sample-attacker").addEventListener("click", () => $("attacker-set").value = sampleAttacker);
  $("sample-defender").addEventListener("click", () => $("defender-set").value = sampleDefender);
  initFieldControls();
  setMode("damage");
  loadMeta();
}

function initFieldControls() {
  document.querySelectorAll("[data-field-key]").forEach((group) => {
    group.querySelectorAll("button").forEach((button) => {
      button.addEventListener("click", () => {
        group.querySelectorAll("button").forEach((peer) => peer.classList.remove("active"));
        button.classList.add("active");
      });
    });
  });
  document.querySelectorAll("[data-field-bool]").forEach((button) => {
    button.classList.add("toggle");
    button.addEventListener("click", () => button.classList.toggle("active"));
  });
  document.querySelectorAll("[data-unsupported]").forEach((node) => {
    node.title = `${node.dataset.unsupported} is visible but not supported by the current damage library Field API.`;
    node.addEventListener("click", (event) => {
      event.preventDefault();
      setFieldNote(`${node.dataset.unsupported}: not supported by current damage lib`);
    });
  });
}

function setMode(mode) {
  state.mode = mode;
  document.querySelectorAll(".tab").forEach((tab) => tab.classList.toggle("active", tab.dataset.mode === mode));
  document.querySelectorAll("[data-field]").forEach((node) => {
    const fields = node.dataset.field.split(/\s+/);
    node.style.display = fields.includes(mode) ? "" : "none";
  });
  if (mode === "ranked") {
    $("summary").textContent = "Full spread search. Lock unused stats to keep it fast.";
  } else {
    $("summary").textContent = "";
  }
}

async function loadMeta() {
  try {
    const meta = await request("/api/meta");
    const list = $("move-list");
    list.replaceChildren(...meta.moves.map((move) => {
      const option = document.createElement("option");
      option.value = move;
      return option;
    }));
  } catch (error) {
    setStatus("Metadata unavailable", true);
  }
}

async function run() {
  setStatus("Running", false, true);
  clearResults();
  const started = performance.now();
  try {
    let data;
    if (state.mode === "damage") {
      data = await request("/api/damage", damagePayload());
      renderDamage(data);
    } else if (state.mode === "survive") {
      data = await request("/api/survive", survivePayload());
      renderSurvival(data);
    } else if (state.mode === "ko") {
      data = await request("/api/ko", koPayload());
      renderKo(data);
    } else {
      const target = $("ranked-mode").value === "offensive" ? "offensive" : "defensive";
      data = await request(`/api/optimize/${target}`, optimizePayload());
      renderRanked(data);
    }
    setStatus(`Done ${(performance.now() - started).toFixed(0)} ms`);
  } catch (error) {
    $("output").innerHTML = `<div class="error-box">${escapeHtml(error.message)}</div>`;
    setStatus("Error", true);
  }
}

async function request(path, payload) {
  const options = payload === undefined ? {} : {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(payload),
  };
  const response = await fetch(path, options);
  const text = await response.text();
  const data = text ? JSON.parse(text) : null;
  if (!response.ok) {
    throw new Error(data?.error || response.statusText);
  }
  return data;
}

function damagePayload() {
  return {
    attacker_set: $("attacker-set").value,
    defender_set: $("defender-set").value,
    move_name: $("move-name").value,
    move_times_affected: number("move-times"),
    field: fieldPayload(),
  };
}

function survivePayload() {
  return {
    ...damagePayload(),
    max_ko_chance: decimal("max-ko"),
    hp_percent: decimal("hp-percent"),
    nature: nature(),
    optimize_nature: $("optimize-nature").checked,
    limit: number("limit"),
  };
}

function koPayload() {
  return {
    ...damagePayload(),
    min_ko_chance: decimal("min-ko"),
    nature: nature(),
    optimize_nature: $("optimize-nature").checked,
    limit: number("limit"),
  };
}

function optimizePayload() {
  return {
    benchmarks: [damagePayload()],
    full_spend: $("full-spend").checked,
    locked: locks(),
    limit: number("limit"),
  };
}

function locks() {
  const out = {};
  document.querySelectorAll("[data-lock]").forEach((input) => {
    out[input.dataset.lock] = input.value === "" ? null : Number(input.value);
  });
  return out;
}

function fieldPayload() {
  return {
    format: selectedFieldValue("format"),
    terrain: selectedFieldValue("terrain"),
    weather: selectedFieldValue("weather"),
    gravity: isFieldActive("gravity"),
    fairy_aura: isFieldActive("fairy_aura"),
    protect: isFieldActive("protect"),
    helping_hand: isFieldActive("helping_hand"),
    attacker_tailwind: isFieldActive("attacker_tailwind"),
    defender_tailwind: isFieldActive("defender_tailwind"),
    defender_reflect: isFieldActive("defender_reflect"),
    defender_light_screen: isFieldActive("defender_light_screen"),
    defender_aurora_veil: isFieldActive("defender_aurora_veil"),
    defender_friend_guard: isFieldActive("defender_friend_guard"),
  };
}

function selectedFieldValue(key) {
  return document.querySelector(`[data-field-key="${key}"] .active`)?.dataset.value || null;
}

function isFieldActive(key) {
  return document.querySelector(`[data-field-bool="${key}"]`)?.classList.contains("active") || false;
}

function number(id) {
  return Math.max(0, Number($(id).value || 0));
}

function decimal(id) {
  return Number($(id).value || 0);
}

function nature() {
  return $("nature").value || null;
}

function renderDamage(data) {
  const row = data.summary;
  $("summary").textContent = `Rolls: ${data.rolls.join(", ")}`;
  table(["Damage", "Percent", "KO chance"], [[
    `${row.min_damage}-${row.max_damage}`,
    `${fmt(row.percent_min)}-${fmt(row.percent_max)}%`,
    chance(row.ko_chance),
  ]]);
}

function renderSurvival(data) {
  $("summary").textContent = data.best ? `Best: ${data.best.sp_line}` : "No satisfying spread";
  renderSpreadRows(data.matches, "result", data.closest_miss);
}

function renderKo(data) {
  $("summary").textContent = data.best ? `Best: ${data.best.sp_line}` : "No satisfying spread";
  renderSpreadRows(data.matches, "result", data.closest_miss, true);
}

function renderRanked(rows) {
  if (!rows.length) {
    $("output").innerHTML = `<div class="empty">No results.</div>`;
    return;
  }
  table(["Rank", "SPs", "Final stats", "Score", "Benchmark 1"], rows.map((row) => [
    row.rank,
    mono(row.sp_line),
    stats(row.final_stats),
    fmt(row.score),
    damageCell(row.results[0]),
  ]));
}

function renderSpreadRows(rows, resultKey, closestMiss, showInvestment = false) {
  const visible = [...rows];
  if (!visible.length && closestMiss) visible.push(closestMiss);
  if (!visible.length) {
    $("output").innerHTML = `<div class="empty">No results.</div>`;
    return;
  }
  const headers = ["Rank", "Nature"];
  if (showInvestment) headers.push("Investment");
  headers.push("SPs", "Total", "Final stats", "Damage", "KO chance");
  table(headers, visible.map((row) => {
    const result = row[resultKey];
    const cells = [row.rank, row.nature];
    if (showInvestment) cells.push(row.investment_stat);
    cells.push(mono(row.sp_line), row.total_points, stats(row.final_stats), damageCell(result), chance(result.ko_chance));
    return cells;
  }));
}

function table(headers, rows) {
  $("output").innerHTML = `<table><thead><tr>${headers.map((h) => `<th>${h}</th>`).join("")}</tr></thead><tbody>${rows.map((row) => `<tr>${row.map((cell) => `<td>${cell}</td>`).join("")}</tr>`).join("")}</tbody></table>`;
}

function damageCell(result) {
  if (!result) return "";
  return `${result.min_damage}-${result.max_damage}<br><span class="mono">${fmt(result.percent_min)}-${fmt(result.percent_max)}%</span>`;
}

function stats(value) {
  return `${value.hp}/${value.attack}/${value.defense}/${value.special_attack}/${value.special_defense}/${value.speed}`;
}

function chance(value) {
  return value === null || value === undefined ? "n/a" : `${fmt(value * 100)}%`;
}

function fmt(value) {
  return Number(value).toFixed(1);
}

function mono(value) {
  return `<span class="mono">${escapeHtml(value)}</span>`;
}

function setStatus(text, isError = false, busy = false) {
  $("status").textContent = text;
  $("status").classList.toggle("error", isError);
  $("status").classList.toggle("busy", busy);
}

function setFieldNote(text) {
  $("field-note").textContent = text;
  window.clearTimeout(state.fieldTimer);
  state.fieldTimer = window.setTimeout(() => $("field-note").textContent = "", 4500);
}

function clearResults() {
  $("output").innerHTML = "";
}

function swapSets() {
  const left = $("attacker-set").value;
  $("attacker-set").value = $("defender-set").value;
  $("defender-set").value = left;
}

function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (char) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
  }[char]));
}

init();
