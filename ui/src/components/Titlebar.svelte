<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Icon from "./Icon.svelte";

  const appWindow = getCurrentWindow();
  let title = "Auera";
</script>

<div class="titlebar glass">
  <div style="display:flex;align-items:center;gap:8px;">
    <span style="font-weight:700;letter-spacing:.5px;">{title}</span>
  </div>
  <div style="display:flex;gap:4px;">
    <button class="ghost" style="padding:4px 8px;" onclick={() => appWindow.minimize()}><Icon name="min" size={14} /></button>
    <button class="ghost" style="padding:4px 8px;" onclick={() => appWindow.toggleMaximize()}><Icon name="max" size={14} /></button>
    <button class="ghost" style="padding:4px 8px;" onclick={() => appWindow.close()}><Icon name="close" size={14} /></button>
  </div>
</div>
