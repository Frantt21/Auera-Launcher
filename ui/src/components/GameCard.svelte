<script>
  import { mediaUrl, glowOf } from "../lib/tauri";
  import Icon from "./Icon.svelte";

  export let game;
  export let mediaBase;
  export let accent = "#e6e6ec";

  export let onClick = () => {};
  export let onFavorite = () => {};
  export let onHover = () => {};
  export let onContext = () => {};

  $: coverUrl = game.art.cover ? mediaUrl(mediaBase, game.art.cover, 336) : null;
  $: glow = /^#[0-9a-f]{6}$/i.test(accent) ? glowOf(accent) : accent;
</script>

<div
  class="game-card"
  style={`--accent:${accent};--glow:${glow};`}
  onclick={() => onClick(game)}
  onmouseenter={() => onHover(game)}
  onmouseleave={() => onHover(null)}
  oncontextmenu={(e) => { e.preventDefault(); onContext(e, game); }}
>
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

