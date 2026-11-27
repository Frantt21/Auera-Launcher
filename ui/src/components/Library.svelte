<script lang="ts">
  import GameCard from "./GameCard.svelte";
  import type { Game, Prefs } from "../lib/types";

  export let games: Game[] = [];
  export let mediaBase = "";
  export let accentFor: (g: Game) => string = () => "#c49cff";
  export let onSelect: (g: Game) => void = () => {};
  export let onFavorite: (g: Game) => void = () => {};
  export let prefs: Prefs = { sort: "recent", source: null, platform: null, favorites_only: false };
  export let onPrefsChange: (p: Prefs) => void = () => {};

  let query = "";
  $: filtered = games.filter((g) =>
    g.title.toLowerCase().includes(query.toLowerCase())
  );

  function update(patch: Partial<Prefs>) {
    const next = { ...prefs, ...patch };
    onPrefsChange(next);
  }
</script>

<div class="scroll" style="height:100%;padding:4px;">
  <div style="display:flex;gap:10px;margin-bottom:14px;align-items:center;flex-wrap:wrap;">
    <input
      bind:value={query}
      placeholder="Buscar..."
      style="flex:1;min-width:160px;padding:8px 12px;border-radius:10px;border:1px solid rgba(255,255,255,0.1);background:rgba(0,0,0,0.35);color:var(--text);"
    />
    <select value={prefs.sort} onchange={(e) => update({ sort: (e.target as HTMLSelectElement).value as any })} style="padding:8px;border-radius:10px;background:rgba(0,0,0,0.35);color:var(--text);border:1px solid rgba(255,255,255,0.1);">
      <option value="recent">Recientes</option>
      <option value="name">Nombre</option>
      <option value="playtime">Tiempo</option>
      <option value="size">Tamaño</option>
    </select>
    <label style="font-size:12px;"><input type="checkbox" checked={prefs.favorites_only} onchange={(e) => update({ favorites_only: (e.target as HTMLInputElement).checked })} /> Favoritos</label>
  </div>
  <div class="card-grid">
    {#each filtered as game (game.id)}
      <GameCard {game} {mediaBase} accent={accentFor(game)} onClick={onSelect} onFavorite={onFavorite} />
    {/each}
  </div>
  {#if filtered.length === 0}
    <div style="opacity:.6;text-align:center;margin-top:40px;">Sin juegos</div>
  {/if}
</div>
