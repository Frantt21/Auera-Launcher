<script>
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Icon from "./Icon.svelte";

  const appWindow = getCurrentWindow();

  function minimize() {
    appWindow.minimize();
  }

  function close() {
    appWindow.close();
  }
</script>

<div class="titlebar" data-tauri-drag-region="deep">
  <div class="window-cluster" data-tauri-drag-region="false">
    <button aria-label="Minimizar" onclick={minimize}><Icon name="min" size={12} /></button>
    <span class="cluster-divider"></span>
    <button class="danger" aria-label="Cerrar" onclick={close}><Icon name="close" size={12} /></button>
  </div>
</div>
