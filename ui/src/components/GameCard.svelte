<script>
  import { mediaUrl } from "../lib/tauri";
  import Icon from "./Icon.svelte";

  export let game;
  export let mediaBase;
  export let accent = "#c49cff";

  export let onClick = () => {};
  export let onFavorite = () => {};

  $: coverUrl = game.art.cover ? mediaUrl(mediaBase, game.art.cover, 336) : null;
</script>

<div class="game-card glass" style={`--accent:${accent};`} onclick={() => onClick(game)}>
  {#if coverUrl}
    <img class="cover" src={coverUrl} alt={game.title} loading="lazy" />
  {:else}
    <div class="cover" style="display:flex;align-items:center;justify-content:center;background:var(--surface-high);">
      <span style="opacity:.5;">{game.title.slice(0,2).toUpperCase()}</span>
    </div>
  {/if}
  <div class="title">{game.title}</div>
  <button
    class="ghost"
    style="position:absolute;top:6px;right:6px;padding:4px 6px;"
    onclick={(e) => { e.stopPropagation(); onFavorite(game); }}
    aria-label="favorite"
  >
    <Icon name="heart" size={14} />
  </button>
  {#if game.favorite}
    <span style="position:absolute;top:8px;left:8px;color:#ffd166;">★</span>
  {/if}
</div>

