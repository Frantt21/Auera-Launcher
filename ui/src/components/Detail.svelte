<script>
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { mediaUrl, accentInk, glowRgb, warmMedia } from "../lib/tauri";
  import Icon from "./Icon.svelte";

  export let game;
  export let mediaBase = "";
  export let accent = "#e6e6ec";
  export let accentRgb = { r: 230, g: 230, b: 236 };
  export let onChanged = () => {};
  // Se puede abrir desde el menú contextual de una tarjeta.
  export let optionsOpen = false;

  const LOGO_WIDTH = 512;

  let busy = false;
  let logoPath = null;
  let logoUrl = null;

  // El título se muestra al instante y el logo entra cuando su miniatura ya
  // está en caché; pedirla sin preparar congelaba la interfaz.
  $: if (game.art.logo !== logoPath) loadLogo(game.art.logo);

  async function loadLogo(path) {
    logoPath = path;
    logoUrl = null;
    if (!path) return;
    await warmMedia(path, LOGO_WIDTH);
    if (logoPath !== path) return;
    logoUrl = mediaUrl(mediaBase, path, LOGO_WIDTH);
  }

  // Editables a mano: el selector nativo devuelve la ruta y el backend la valida.
  async function pickPrefix() {
    const path = await open({ directory: true, multiple: false, title: "Elegir prefijo Wine" });
    if (typeof path !== "string") return;
    await invoke("pick_prefix", { id: game.id, path });
    onChanged();
  }

  async function pickExecutable() {
    const path = await open({ multiple: false, title: "Elegir ejecutable del juego" });
    if (typeof path !== "string") return;
    await invoke("pick_executable", { id: game.id, path });
    onChanged();
  }

  // Mismo color que el halo de las tarjetas, para que el botón no herede acentos apagados.
  $: glowRgbValue = /^#[0-9a-f]{6}$/i.test(accent) ? glowRgb(accent) : accentRgb;
  $: glow = `rgb(${glowRgbValue.r}, ${glowRgbValue.g}, ${glowRgbValue.b})`;
  $: glowInk = accentInk(glowRgbValue);
  $: playtime =
    game.playtime_min === 0
      ? "—"
      : game.playtime_min < 60
        ? `${game.playtime_min} min`
        : `${Math.floor(game.playtime_min / 60)} h ${game.playtime_min % 60} min`;
  $: size = game.size_bytes ? `${(game.size_bytes / 1073741824).toFixed(1)} GB` : "—";

  async function play() {
    busy = true;
    try { await invoke("play", { id: game.id }); }
    catch (e) { alert(String(e)); }
    finally { busy = false; onChanged(); }
  }
</script>

<div class="detail">
  <div class="detail-main">
    {#if logoUrl}
      <img class="detail-logo" src={logoUrl} alt={game.title} />
    {:else}
      <h1 class="detail-title">{game.title}</h1>
    {/if}
    <div class="detail-actions">
      <button
        class="detail-play"
        style={`--glow:${glow};--glow-ink:${glowInk};`}
        disabled={busy || game.blocked}
        onclick={play}
      >
        Jugar
      </button>
      <button class="detail-options" onclick={() => (optionsOpen = true)} aria-label="Opciones">
        <Icon name="settings" size={20} />
      </button>
    </div>
  </div>

  <div class="detail-info glass">
    <div><span>Horas jugadas</span><b>{playtime}</b></div>
    <div><span>Peso</span><b>{size}</b></div>
    <div><span>Plataforma</span><b class="detail-platform">{game.platform}</b></div>
  </div>
</div>

{#if optionsOpen}
  <div class="modal-backdrop" onclick={() => (optionsOpen = false)}>
    <div class="modal glass" onclick={(e) => e.stopPropagation()}>
      <h3 style="margin-top:0;">Opciones</h3>
      <dl class="game-paths">
        <dt>Ejecutable</dt>
        <dd class:empty={!game.executable}>{game.executable ?? "Sin detectar"}</dd>
        <dt>Prefijo</dt>
        <dd class:empty={!game.prefix}>{game.prefix ?? "Sin detectar"}</dd>
        {#if game.proton}
          <dt>Proton</dt>
          <dd>{game.proton}</dd>
        {/if}
      </dl>
      <div style="display:flex;gap:8px;flex-wrap:wrap;">
        <button class="ghost" onclick={pickPrefix}>Elegir prefijo…</button>
        <button class="ghost" onclick={pickExecutable}>Elegir ejecutable…</button>
      </div>
      <div style="display:flex;gap:8px;flex-wrap:wrap;margin-top:8px;">
        <button class="ghost" onclick={async () => { await invoke("detect_prefix", { id: game.id }); onChanged(); }}>Detectar prefijo</button>
        <button class="ghost" onclick={async () => { await invoke("create_prefix", { id: game.id, protonLabel: game.proton ?? "" }); onChanged(); }}>Crear prefijo</button>
      </div>
      <div style="margin-top:12px;">
        <button class="ghost" onclick={async () => {
          const q = prompt("Buscar arte (nombre):", game.title);
          if (!q) return;
          await invoke("search_art", { id: game.id, query: q });
        }}>Buscar arte</button>
      </div>
      <div style="margin-top:16px;text-align:right;">
        <button class="ghost" onclick={() => (optionsOpen = false)}>Cerrar</button>
      </div>
    </div>
  </div>
{/if}
