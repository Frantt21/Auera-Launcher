<script>
  import { tick } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Dropdown from "./Dropdown.svelte";
  import Icon from "./Icon.svelte";

  export let query = "";
  export let onQuery = () => {};
  export let prefs = { sort: "recent", favorites_only: false };
  export let onPrefsChange = () => {};
  export let back = false;
  export let onBack = () => {};

  const sorts = [
    { value: "recent", label: "Recientes" },
    { value: "name", label: "Nombre" },
    { value: "playtime", label: "Tiempo" },
    { value: "size", label: "Tamaño" },
  ];

  $: extras = [
    {
      label: "Solo favoritos",
      active: prefs.favorites_only,
      onSelect: () => update({ favorites_only: !prefs.favorites_only }),
    },
  ];

  function update(patch) {
    onPrefsChange({ ...prefs, ...patch });
  }

  const appWindow = getCurrentWindow();

  let open = false;
  let inputEl = null;

  function minimize() {
    appWindow.minimize();
  }

  function close() {
    appWindow.close();
  }

  async function toggleSearch() {
    if (open) {
      open = false;
      onQuery("");
      return;
    }
    open = true;
    await tick();
    inputEl?.focus();
  }
</script>

<div class="titlebar" data-tauri-drag-region="deep">
  {#if back}
    <button class="title-back" data-tauri-drag-region="false" onclick={onBack} aria-label="Volver">
      <Icon name="back" size={16} />
    </button>
  {/if}

  <div class="title-search" class:open data-tauri-drag-region="false">
    <input
      bind:this={inputEl}
      value={query}
      placeholder="Buscar..."
      oninput={(e) => onQuery(e.currentTarget.value)}
      onkeydown={(e) => { if (e.key === "Escape") toggleSearch(); }}
    />
    <button class="title-search-btn" onclick={toggleSearch} aria-label={open ? "Cerrar búsqueda" : "Buscar"}>
      <Icon name="search" size={14} />
    </button>
    <Dropdown value={prefs.sort} options={sorts} onChange={(sort) => update({ sort })} {extras} />
  </div>

  <div class="window-cluster" data-tauri-drag-region="false">
    <button aria-label="Minimizar" onclick={minimize}><Icon name="min" size={12} /></button>
    <span class="cluster-divider"></span>
    <button class="danger" aria-label="Cerrar" onclick={close}><Icon name="close" size={12} /></button>
  </div>
</div>
