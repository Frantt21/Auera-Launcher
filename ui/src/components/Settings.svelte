<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { Config, Bootstrap } from "../lib/types";

  export let cfg: Config;
  export let log: string[] = [];
  export let busy: { scanning: boolean; art: boolean; tiers: boolean; prefix: boolean } = { scanning: false, art: false, tiers: false, prefix: false };
  export let onScan: () => void = () => {};
  export let onFetchArt: () => void = () => {};
  export let onFetchTiers: () => void = () => {};

  async function save() {
    await invoke("set_config", { cfg });
  }
  async function addFolder() {
    const p = prompt("Ruta de carpeta:");
    if (!p) return;
    cfg.watched_folders = [...cfg.watched_folders, p];
    await save();
  }
</script>

<div class="scroll" style="height:100%;padding:4px;display:grid;gap:16px;">
  <div class="glass" style="padding:16px;">
    <h3 style="margin:0 0 10px;">Escaneo</h3>
    <label><input type="checkbox" bind:checked={cfg.scan_steam} onchange={save} /> Steam</label><br />
    <label><input type="checkbox" bind:checked={cfg.scan_epic} onchange={save} /> Epic</label><br />
    <label><input type="checkbox" bind:checked={cfg.scan_heroic} onchange={save} /> Heroic</label><br />
    <label><input type="checkbox" bind:checked={cfg.scan_hydra} onchange={save} /> Hydra</label><br />
    <label><input type="checkbox" bind:checked={cfg.scan_desktop} onchange={save} /> Desktop</label><br />
    <button class="primary" style="margin-top:10px;" disabled={busy.scanning} onclick={onScan}>Escanear ahora</button>
  </div>

  <div class="glass" style="padding:16px;">
    <h3 style="margin:0 0 10px;">Integraciones</h3>
    <label>SteamGridDB key<br />
      <input bind:value={cfg.steamgrid_key} onchange={save} style="width:100%;padding:6px;border-radius:8px;border:1px solid rgba(255,255,255,0.1);background:rgba(0,0,0,0.35);color:var(--text);" />
    </label><br />
    <label><input type="checkbox" bind:checked={cfg.steamgrid_enabled} onchange={save} /> SteamGridDB activo</label><br />
    <label><input type="checkbox" bind:checked={cfg.protondb_enabled} onchange={save} /> ProtonDB activo</label><br />
    <div style="display:flex;gap:8px;margin-top:10px;">
      <button class="ghost" disabled={busy.art} onclick={onFetchArt}>Sincronizar arte</button>
      <button class="ghost" disabled={busy.tiers} onclick={onFetchTiers}>Sincronizar tiers</button>
    </div>
  </div>

  <div class="glass" style="padding:16px;">
    <h3 style="margin:0 0 10px;">Sistema</h3>
    <label><input type="checkbox" bind:checked={cfg.gamepad_enabled} onchange={save} /> Mando (gamepad)</label><br />
    <h4>Carpetas vigiladas</h4>
    <ul>
      {#each cfg.watched_folders as f}
        <li style="font-size:12px;">{f}</li>
      {/each}
    </ul>
    <button class="ghost" onclick={addFolder}>Añadir carpeta</button>
  </div>

  <div class="glass" style="padding:16px;">
    <h3 style="margin:0 0 10px;">Log</h3>
    <div class="scroll" style="max-height:200px;font-size:11px;opacity:.8;">
      {#each log as line}<div>{line}</div>{/each}
    </div>
  </div>
</div>
