<script>
  import Icon from "./Icon.svelte";
  export let view;
  export let onNavigate;
  export let gamepadName = null;
</script>

<div class="rail glass">
  <button class="ghost" style="width:100%;" onclick={() => onNavigate("library")} class:active={view === "library"}>
    <Icon name="library" size={20} />
  </button>
  <button class="ghost" style="width:100%;" onclick={() => onNavigate("settings")} class:active={view === "settings"}>
    <Icon name="settings" size={20} />
  </button>
  <div style="flex:1;"></div>
  {#if gamepadName}
    <div title={gamepadName} style="opacity:.8;"><Icon name="gamepad" size={18} /></div>
  {/if}
</div>

