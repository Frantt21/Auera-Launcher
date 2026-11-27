<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import Titlebar from "./components/Titlebar.svelte";
  import Rail from "./components/Rail.svelte";
  import Library from "./components/Library.svelte";
  import Detail from "./components/Detail.svelte";
  import Settings from "./components/Settings.svelte";
  import Toasts from "./components/Toasts.svelte";
  import Icon from "./components/Icon.svelte";
  import { mediaUrl, rgbToHex } from "./lib/tauri";
  import type { Game, Config, Prefs, Bootstrap, BackdropInfo, Busy, Candidates } from "./lib/types";

  let cfg: Config | null = null;
  let prefs: Prefs = { sort: "recent", source: null, platform: null, favorites_only: false };
  let games: Game[] = [];
  let protons: any[] = [];
  let mediaBase = "";
  let busy: Busy = { scanning: false, art: false, tiers: false, prefix: false };
  let log: string[] = [];
  let toasts: string[] = [];

  let view: "library" | "detail" | "settings" = "library";
  let selectedId: string | null = null;
  let selectedGame: Game | null = null;
  let selectedAccent = "#c49cff";
  let selectedAccentRgb = { r: 196, g: 156, b: 255 };
  let backdropUrl: string | null = null;
  let lumaGrid: { width: number; height: number; values: number[] } | null = null;
  let toneClass = "";

  let gamepadName: string | null = null;
  let candidates: Candidates | null = null;

  let accentMap: Record<string, string> = {};
  const fetchedAccents = new Set<string>();
  let accentQueue: string[] = [];
  let accentWorkers = 0;
  let fatal: string | null = null;

  $: selectedGame = games.find((g) => g.id === selectedId) ?? null;
  $: visibleGames = sortGames(games, prefs);
  $: if (selectedGame) loadSelected(selectedGame.id);

  function sortGames(list: Game[], p: Prefs): Game[] {
    let out = [...list];
    if (p.source) out = out.filter((g) => g.source === p.source);
    if (p.platform) out = out.filter((g) => g.platform === p.platform);
    if (p.favorites_only) out = out.filter((g) => g.favorite);
    out.sort((a, b) => {
      switch (p.sort) {
        case "name": return a.title.localeCompare(b.title);
        case "recent": return (b.last_played ?? 0) - (a.last_played ?? 0);
        case "playtime": return b.playtime_min - a.playtime_min;
        case "size": return (b.size_bytes ?? 0) - (a.size_bytes ?? 0);
        default: return 0;
      }
    });
    return out;
  }

  function pushToast(msg: string) {
    toasts = [...toasts, msg];
    setTimeout(() => { toasts = toasts.slice(1); }, 3500);
  }

  async function loadSelected(id: string) {
    try {
      const rgb = await invoke<{ r: number; g: number; b: number }>("accent", { id, preferLogo: true });
      selectedAccentRgb = rgb;
      selectedAccent = rgbToHex(rgb.r, rgb.g, rgb.b);
    } catch {}
    try {
      const info = await invoke<BackdropInfo | null>("backdrop", { id });
      if (info) {
        backdropUrl = info.url;
        lumaGrid = info.luma;
        const avg = info.luma.values.reduce((a, b) => a + b, 0) / info.luma.values.length;
        toneClass = avg >= 150 ? "tone-light" : "";
      } else {
        backdropUrl = null;
        lumaGrid = null;
        toneClass = "";
      }
    } catch {}
  }

  async function fetchAccents() {
    for (const g of games) {
      if (fetchedAccents.has(g.id)) continue;
      fetchedAccents.add(g.id);
      accentQueue.push(g.id);
    }
    // Pocos a la vez: cada análisis decodifica una imagen en el backend.
    while (accentWorkers < 4 && accentQueue.length > 0) {
      const id = accentQueue.shift()!;
      accentWorkers++;
      invoke<{ r: number; g: number; b: number }>("accent", { id, preferLogo: false })
        .then((rgb) => { accentMap[id] = rgbToHex(rgb.r, rgb.g, rgb.b); accentMap = { ...accentMap }; })
        .catch(() => {})
        .finally(() => { accentWorkers--; fetchAccents(); });
    }
  }

  onMount(async () => {
    try {
      const b = await invoke<Bootstrap>("bootstrap");
      cfg = b.cfg;
      prefs = b.prefs;
      games = b.games;
      protons = b.protons;
      mediaBase = b.media_base;
      busy = b.busy;
      log = b.log;
    } catch (e) {
      fatal = String(e);
      return;
    }

    await listen<Game[]>("library", (e) => { games = e.payload; fetchAccents(); });
    await listen<Busy>("busy", (e) => { busy = e.payload; });
    await listen<string[]>("log", (e) => { log = e.payload; });
    await listen<string>("toast", (e) => { pushToast(e.payload); });
    await listen<Candidates>("candidates", (e) => { candidates = e.payload; });
    await listen<string>("gamepad-name", (e) => { gamepadName = e.payload; });
    await listen<any>("gamepad", (e) => { handleGamepad(e.payload); });

    fetchAccents();
  });

  function handleGamepad(nav: any) {
    // basic navigation: map to view changes not implemented fully
  }

  async function scan() {
    await invoke("scan");
  }
  async function fetchArt() {
    await invoke("fetch_art", { only: null });
  }
  async function fetchTiers() {
    await invoke("fetch_tiers");
  }

  async function onFavorite(g: Game) {
    await invoke("toggle_favorite", { id: g.id });
  }
  async function onChanged() {
    // refresh game list
    const b = await invoke<Bootstrap>("bootstrap");
    games = b.games;
  }

  async function applyPrefs(p: Prefs) {
    prefs = p;
    await invoke("set_prefs", { prefs: p });
  }
  async function applyCfg(c: Config) {
    cfg = c;
    await invoke("set_config", { cfg: c });
  }

  async function pickCandidate(sgdbId: number) {
    if (!candidates) return;
    await invoke("pick_art", { id: candidates.id, sgdbId });
    candidates = null;
  }
</script>

<div class={toneClass} style="height:100%;position:relative;">
  {#if backdropUrl}
    <img class="backdrop" src={backdropUrl} alt="" />
    <div class="backdrop-dim"></div>
  {/if}

  <Titlebar />
  <Rail {view} onNavigate={(v) => (view = v as any)} {gamepadName} />

  <div class="content">
    {#if fatal}
      <div class="glass" style="padding:20px;">
        <strong>No se pudo iniciar la interfaz</strong>
        <div class="muted" style="margin-top:8px;font-size:13px;">{fatal}</div>
      </div>
    {:else if view === "library"}
      <Library
        games={visibleGames}
        {mediaBase}
        accentFor={(g) => accentMap[g.id] ?? "#c49cff"}
        onSelect={(g) => { selectedId = g.id; view = "detail"; }}
        onFavorite={onFavorite}
        prefs={prefs}
        onPrefsChange={(p) => applyPrefs(p)}
      />
    {:else if view === "detail" && selectedGame}
      <Detail
        game={selectedGame}
        {mediaBase}
        accent={selectedAccent}
        accentRgb={selectedAccentRgb}
        onBack={() => (view = "library")}
        onChanged={onChanged}
      />
    {:else if view === "settings" && cfg}
      <Settings
        cfg={cfg}
        {log}
        {busy}
        onScan={scan}
        onFetchArt={fetchArt}
        onFetchTiers={fetchTiers}
      />
    {/if}
  </div>

  {#if candidates}
    <div class="modal-backdrop" onclick={() => (candidates = null)}>
      <div class="modal glass" onclick={(e) => e.stopPropagation()}>
        <h3 style="margin-top:0;">Resultados</h3>
        {#if candidates.results.length === 0}
          <div>Sin resultados</div>
        {:else}
          {#each candidates.results as [id, name]}
            <button class="ghost" style="display:block;width:100%;text-align:left;margin:4px 0;" onclick={() => pickCandidate(id)}>
              {name} (#{id})
            </button>
          {/each}
        {/if}
        <div style="text-align:right;margin-top:10px;">
          <button class="ghost" onclick={() => (candidates = null)}>Cerrar</button>
        </div>
      </div>
    </div>
  {/if}

  <Toasts {toasts} />
</div>

<style>
  /* bind cfg changes to save handled in Settings via set_config */
</style>
