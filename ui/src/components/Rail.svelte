<script>
  import Icon from "./Icon.svelte";
  import { mediaUrl } from "../lib/tauri";

  export let view;
  export let onNavigate;
  export let gamepadName = null;
  export let favorites = [];
  export let mediaBase = "";
  export let onOpen = () => {};
  export let onHover = () => {};
  export let onContext = () => {};
</script>

<div class="rail glass">
  <button class="ghost" onclick={() => onNavigate("library")} class:active={view === "library"}>
    <Icon name="library" size={20} />
  </button>
  <button class="ghost" onclick={() => onNavigate("settings")} class:active={view === "settings"}>
    <Icon name="settings" size={20} />
  </button>
  <div class="rail-favorites">
    {#each favorites as game (game.id)}
      <button
        class="rail-fav"
        onclick={() => onOpen(game)}
        onmouseenter={() => onHover(game)}
        onmouseleave={() => onHover(null)}
        oncontextmenu={(e) => { e.preventDefault(); onContext(e, game); }}
        title={game.title}
      >
        {#if game.art.cover}
          <img src={mediaUrl(mediaBase, game.art.cover, 104)} alt={game.title} loading="lazy" />
        {:else}
          <span>{game.title.slice(0, 2).toUpperCase()}</span>
        {/if}
      </button>
    {/each}
  </div>
  {#if gamepadName}
    <div title={gamepadName} style="opacity:.8;"><Icon name="gamepad" size={18} /></div>
  {/if}
</div>
