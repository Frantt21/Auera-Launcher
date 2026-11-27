<script lang="ts">
  export let toasts: string[] = [];
</script>

<div class="toast-stack">
  {#each toasts as t}
    <div class="toast">{t}</div>
  {/each}
</div>
