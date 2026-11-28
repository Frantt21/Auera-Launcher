<script>
  import GameCard from "./GameCard.svelte";

  export let games = [];
  export let mediaBase = "";
  export let accentFor = () => "#e6e6ec";
  export let onSelect = () => {};
  export let onHover = () => {};
  export let onContext = () => {};
  export let query = "";

  $: filtered = games.filter((g) =>
    g.title.toLowerCase().includes(query.toLowerCase())
  );
</script>

<div class="scroll" style="height:100%;padding:8px 20px 30px 20px;">
  <div class="card-grid">
    {#each filtered as game (game.id)}
      <GameCard {game} {mediaBase} accent={accentFor(game)} onClick={onSelect} {onHover} {onContext} />
    {/each}
  </div>
  {#if filtered.length === 0}
    <div style="opacity:.6;text-align:center;margin-top:40px;">Sin juegos</div>
  {/if}
</div>
