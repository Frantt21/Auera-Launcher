<script>
  export let toasts = [];
</script>

<div class="toast-stack">
  {#each toasts as t}
    <div class="toast">{t}</div>
  {/each}
</div>
