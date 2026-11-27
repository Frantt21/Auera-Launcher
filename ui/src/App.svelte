<script>
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
  import { rgbToHex } from "./lib/tauri";

  let cfg = null;
  let prefs = { sort: "recent", source: null, platform: null, favorites_only: false };
  let games = [];
  let protons = [];
  let mediaBase = "";
  let busy = { scanning: false, art: false, tiers: false, prefix: false };
  let log = [];
  let toasts = [];

  let view = "library";
  let selectedId = null;
  let selectedGame = null;
  let selectedAccent = "#c49cff";
  let selectedAccentRgb = { r: 196, g: 156, b: 255 };

  // Fondo del juego enfocado: dos capas para cruzar con fundido.
  let bgCurrent = null;
  let bgPrevious = null;
  let bgTimer = null;
  let bgKey = null;
  let hoverTimer = null;

  let gamepadName = null;
  let candidates = null;
  let contextMenu = null;
  let pendingDelete = null;

  let accentMap = {};
  const fetchedAccents = new Set();
  let accentQueue = [];
  let accentWorkers = 0;
  let fatal = null;

  $: selectedGame = games.find((g) => g.id === selectedId) ?? null;
  $: visibleGames = sortGames(games, prefs);
  $: favoriteGames = visibleGames.filter((g) => g.favorite);
  $: if (view === "detail" && selectedGame) loadSelected(selectedGame.id);

  function sortGames(list, p) {
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

  function pushToast(msg) {
    toasts = [...toasts, msg];
    setTimeout(() => { toasts = toasts.slice(1); }, 3500);
  }

  async function loadSelected(id) {
    showBackdrop(id);
    try {
      const rgb = await invoke("accent", { id, preferLogo: true });
      selectedAccentRgb = rgb;
      selectedAccent = rgbToHex(rgb.r, rgb.g, rgb.b);
    } catch {}
  }

  function setBackdrop(url) {
    clearTimeout(bgTimer);
    bgPrevious = bgCurrent;
    bgCurrent = url;
    if (bgPrevious) bgTimer = setTimeout(() => { bgPrevious = null; }, 420);
  }

  async function showBackdrop(id) {
    clearTimeout(hoverTimer);
    if (id === bgKey) return;
    bgKey = id;
    if (!id) {
      setBackdrop(null);
      return;
    }
    const url = await invoke("backdrop", { id }).catch(() => null);
    if (id !== bgKey) return;
    setBackdrop(url);
  }

  // Retrasa la carga del fondo mientras el ratón barre varias tarjetas.
  function hoverBackdrop(id) {
    clearTimeout(hoverTimer);
    if (!id) {
      showBackdrop(null);
      return;
    }
    if (id === bgKey) return;
    hoverTimer = setTimeout(() => showBackdrop(id), 60);
  }

  // Prepara los fondos de la primera pantalla para que el primer hover sea inmediato.
  async function warmBackdrops() {
    for (const g of games.slice(0, 20)) {
      if (!g.art?.hero && !g.art?.cover) continue;
      await invoke("backdrop", { id: g.id }).catch(() => null);
    }
  }

  async function fetchAccents() {
    for (const g of games) {
      if (fetchedAccents.has(g.id)) continue;
      fetchedAccents.add(g.id);
      accentQueue.push(g.id);
    }
    // Pocos a la vez: cada análisis decodifica una imagen en el backend.
    while (accentWorkers < 4 && accentQueue.length > 0) {
      const id = accentQueue.shift();
      accentWorkers++;
      invoke("accent", { id, preferLogo: false })
        .then((rgb) => { accentMap[id] = rgbToHex(rgb.r, rgb.g, rgb.b); accentMap = { ...accentMap }; })
        .catch(() => {})
        .finally(() => { accentWorkers--; fetchAccents(); });
    }
  }

  onMount(async () => {
    try {
      const b = await invoke("bootstrap");
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

    await listen("library", (e) => { games = e.payload; fetchAccents(); });
    await listen("busy", (e) => { busy = e.payload; });
    await listen("log", (e) => { log = e.payload; });
    await listen("toast", (e) => { pushToast(e.payload); });
    await listen("candidates", (e) => { candidates = e.payload; });
    await listen("gamepad-name", (e) => { gamepadName = e.payload; });
    await listen("gamepad", (e) => { handleGamepad(e.payload); });

    fetchAccents();
    setTimeout(warmBackdrops, 1200);
  });

  function handleGamepad(nav) {
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

  function openContext(e, game) {
    contextMenu = {
      game,
      x: Math.min(e.clientX, window.innerWidth - 190),
      y: Math.min(e.clientY, window.innerHeight - 92),
    };
  }

  async function refreshBackdrop() {
    const game = contextMenu.game;
    contextMenu = null;
    const url = await invoke("refresh_backdrop", { id: game.id }).catch(() => null);
    if (!url) return;
    bgKey = game.id;
    setBackdrop(url);
  }

  async function deleteGame() {
    const game = pendingDelete;
    pendingDelete = null;
    await invoke("remove_game", { id: game.id });
    if (selectedId === game.id) {
      selectedId = null;
      view = "library";
    }
    showBackdrop(null);
  }

  async function onFavorite(g) {
    await invoke("toggle_favorite", { id: g.id });
  }
  async function onChanged() {
    // refresh game list
    const b = await invoke("bootstrap");
    games = b.games;
  }

  async function applyPrefs(p) {
    prefs = p;
    await invoke("set_prefs", { prefs: p });
  }
  async function applyCfg(c) {
    cfg = c;
    await invoke("set_config", { cfg: c });
  }

  async function pickCandidate(sgdbId) {
    if (!candidates) return;
    await invoke("pick_art", { id: candidates.id, sgdbId });
    candidates = null;
  }
</script>

<div class="shell">
  {#if bgPrevious}
    <img class="backdrop fade-out" src={bgPrevious} alt="" />
  {/if}
  {#if bgCurrent}
    <img class="backdrop fade-in" src={bgCurrent} alt="" />
  {/if}
  {#if bgCurrent || bgPrevious}
    <div class="backdrop-dim"></div>
  {/if}

  <Titlebar />
  <Rail
    {view}
    onNavigate={(v) => { view = v; if (v !== "detail") showBackdrop(null); }}
    {gamepadName}
    favorites={favoriteGames}
    {mediaBase}
    onOpen={(g) => { selectedId = g.id; view = "detail"; }}
    onHover={(g) => hoverBackdrop(g ? g.id : null)}
    onContext={openContext}
  />

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
        onHover={(g) => hoverBackdrop(g ? g.id : null)}
        onFavorite={onFavorite}
        prefs={prefs}
        onPrefsChange={(p) => applyPrefs(p)}
        onContext={openContext}
      />
    {:else if view === "detail" && selectedGame}
      <Detail
        game={selectedGame}
        {mediaBase}
        accent={selectedAccent}
        accentRgb={selectedAccentRgb}
        onBack={() => { view = "library"; showBackdrop(null); }}
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

  {#if contextMenu}
    <div
      class="ctx-scrim"
      onclick={() => (contextMenu = null)}
      oncontextmenu={(e) => { e.preventDefault(); contextMenu = null; }}
    ></div>
    <div class="context-menu glass" style={`top:${contextMenu.y}px;left:${contextMenu.x}px;`}>
      <button onclick={refreshBackdrop}>Recalcular hover</button>
      <button class="danger" onclick={() => { pendingDelete = contextMenu.game; contextMenu = null; }}>Eliminar</button>
    </div>
  {/if}

  {#if pendingDelete}
    <div class="modal-backdrop" onclick={() => (pendingDelete = null)}>
      <div class="modal glass" onclick={(e) => e.stopPropagation()}>
        <h3 style="margin-top:0;">Eliminar juego</h3>
        <div style="font-size:13px;opacity:.8;">
          Se quitará «{pendingDelete.title}» de la biblioteca. Los archivos instalados no se tocan.
        </div>
        <div style="display:flex;justify-content:flex-end;gap:8px;margin-top:16px;">
          <button class="ghost" onclick={() => (pendingDelete = null)}>Cancelar</button>
          <button class="danger" onclick={deleteGame}>Eliminar</button>
        </div>
      </div>
    </div>
  {/if}

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

