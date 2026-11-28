<script>
  import Icon from "./Icon.svelte";

  export let value;
  export let options = [];
  export let onChange = () => {};
  // Acciones sueltas bajo el separador, como marcar solo favoritos.
  export let extras = [];

  const MENU_WIDTH = 170;

  let open = false;
  let toggle;
  let anchor = { top: 0, left: 0 };

  $: current = options.find((opt) => opt.value === value)?.label ?? "";

  // El menú va en coordenadas de ventana para que el scroll de la lista no lo recorte.
  function show() {
    const rect = toggle.getBoundingClientRect();
    const left = Math.max(12, Math.min(rect.left, window.innerWidth - MENU_WIDTH - 12));
    anchor = { top: rect.bottom + 6, left };
    open = true;
  }

  function run(extra) {
    open = false;
    extra.onSelect();
  }

  function choose(next) {
    open = false;
    if (next !== value) onChange(next);
  }
</script>

<div class="dropdown">
  <button bind:this={toggle} class="dropdown-toggle" onclick={() => (open ? (open = false) : show())}>
    <span>{current}</span>
    <Icon name="chevron" size={12} />
  </button>
  {#if open}
    <div class="dropdown-scrim" onclick={() => (open = false)}></div>
    <div class="dropdown-menu glass" style={`top:${anchor.top}px;left:${anchor.left}px;`}>
      {#each options as opt (opt.value)}
        <button class:active={opt.value === value} onclick={() => choose(opt.value)}>{opt.label}</button>
      {/each}
      {#if extras.length}
        <div class="dropdown-divider"></div>
        {#each extras as extra}
          <button class="dropdown-extra" class:active={extra.active} onclick={() => run(extra)}>
            <Icon name="check" size={12} />
            <span>{extra.label}</span>
          </button>
        {/each}
      {/if}
    </div>
  {/if}
</div>
