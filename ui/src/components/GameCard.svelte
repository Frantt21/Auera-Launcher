<script>
  import { mediaUrl, glowOf } from "../lib/tauri";

  export let game;
  export let mediaBase;
  export let accent = "#e6e6ec";

  export let onClick = () => {};
  export let onHover = () => {};
  export let onContext = () => {};

  $: coverUrl = game.art.cover ? mediaUrl(mediaBase, game.art.cover, 336) : null;
  $: glow = /^#[0-9a-f]{6}$/i.test(accent) ? glowOf(accent) : accent;
</script>

<div
  class="game-card"
  style={`--glow:${glow};`}
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
  {#if game.favorite}
    <span style="position:absolute;top:8px;left:8px;color:#ffd166;">★</span>
  {/if}
</div>

