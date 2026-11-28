<script>
  import { invoke } from "@tauri-apps/api/core";
  import { mediaUrl, accentInk } from "../lib/tauri";
  import Icon from "./Icon.svelte";

  export let game;
  export let mediaBase = "";
  export let accent = "#e6e6ec";
  export let accentRgb = { r: 230, g: 230, b: 236 };
  export let onBack = () => {};
  export let onChanged = () => {};

  let busy = false;
  let optionsOpen = false;

  $: logoUrl = game.art.logo ? mediaUrl(mediaBase, game.art.logo, 512) : null;
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
  <button class="detail-back" onclick={onBack} aria-label="Volver">
    <Icon name="back" size={18} />
  </button>

  <div class="detail-main">
    {#if logoUrl}
      <img class="detail-logo" src={logoUrl} alt={game.title} />
    {:else}
      <h1 class="detail-title">{game.title}</h1>
    {/if}
    <div class="detail-actions">
      <button
        class="detail-play"
        style={`--btn-bg:${accent};--btn-ink:${accentInk(accentRgb)};`}
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
      <p style="opacity:.7;font-size:13px;">Prefijo, Proton y arte se gestionan desde aquí.</p>
      <div style="display:flex;gap:8px;flex-wrap:wrap;">
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
