<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { mediaUrl, rgbToHex, accentInk } from "../lib/tauri";
  import type { Game, Bootstrap } from "../lib/types";
  import Icon from "./Icon.svelte";

  export let game: Game;
  export let mediaBase: string = "";
  export let accent: string = "#c49cff";
  export let accentRgb: { r: number; g: number; b: number } = { r: 196, g: 156, b: 255 };
  export let onBack: () => void = () => {};
  export let onChanged: () => void = () => {};

  let busy = false;
  let optionsOpen = false;

  $: coverUrl = game.art.cover ? mediaUrl(mediaBase, game.art.cover, 640) : null;
  $: logoUrl = game.art.logo ? mediaUrl(mediaBase, game.art.logo, 256) : null;

  async function play() {
    busy = true;
    try { await invoke("play", { id: game.id }); }
    catch (e) { alert(String(e)); }
    finally { busy = false; onChanged(); }
  }
  async function toggleFav() {
    await invoke("toggle_favorite", { id: game.id });
    onChanged();
  }
  async function computeSize() {
    await invoke("compute_size", { id: game.id });
  }
</script>

<div class="scroll" style="height:100%;padding:4px;">
  <button class="ghost" onclick={onBack} style="margin-bottom:12px;">← Volver</button>

  <div class="glass" style="padding:16px;display:grid;grid-template-columns:200px 1fr;gap:16px;">
    {#if coverUrl}
      <img src={coverUrl} alt={game.title} style="width:200px;height:300px;object-fit:cover;border-radius:10px;" />
    {:else}
      <div style="width:200px;height:300px;background:var(--surface-high);border-radius:10px;display:flex;align-items:center;justify-content:center;">{game.title.slice(0,2)}</div>
    {/if}
    <div>
      <h1 style="margin:0 0 6px;font-size:24px;">{game.title}</h1>
      <div style="opacity:.7;margin-bottom:10px;">{game.source} · {game.platform}</div>
      <div style="display:flex;gap:8px;flex-wrap:wrap;margin-bottom:12px;">
        <span class="glass" style="padding:4px 8px;font-size:12px;">Tier: {game.tier}</span>
        <span class="glass" style="padding:4px 8px;font-size:12px;">Tiempo: {game.playtime_min} min</span>
        <span class="glass" style="padding:4px 8px;font-size:12px;">Tamaño: {game.size_bytes ? (game.size_bytes/1073741824).toFixed(1)+" GB" : "—"}</span>
      </div>
      {#if game.prefix}
        <div style="font-size:12px;opacity:.7;margin-bottom:4px;">Prefijo: {game.prefix}</div>
      {/if}
      {#if game.proton}
        <div style="font-size:12px;opacity:.7;margin-bottom:10px;">Proton: {game.proton}</div>
      {/if}
      <div style="display:flex;gap:8px;flex-wrap:wrap;">
        <button class="primary" style={`background:${accent};color:${accentInk(accentRgb)};`} disabled={busy} onclick={play}>
          <Icon name="play" size={14} /> Jugar
        </button>
        <button class="ghost" onclick={toggleFav}>
          <Icon name="heart" size={14} /> {game.favorite ? "Quitar favorito" : "Favorito"}
        </button>
        <button class="ghost" onclick={() => (optionsOpen = true)}>Opciones</button>
        <button class="ghost" onclick={computeSize}>Calcular tamaño</button>
      </div>
    </div>
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

