export type Platform = "windows" | "linux";
export type Source = "steam" | "epic" | "heroic" | "hydra" | "desktop" | "manual";
export type Tier = "unknown" | "borked" | "bronze" | "silver" | "gold" | "platinum";
export type Sort = "name" | "recent" | "playtime" | "size";

export interface Art {
  hero: string | null;
  cover: string | null;
  logo: string | null;
  icon: string | null;
}

export interface Game {
  id: string;
  title: string;
  source: Source;
  app_id: number | null;
  executable: string | null;
  install_dir: string;
  platform: Platform;
  platform_locked: boolean;
  blocked: boolean;
  prefix: string | null;
  proton: string | null;
  size_bytes: number | null;
  playtime_min: number;
  last_played: number | null;
  favorite: boolean;
  art: Art;
  sgdb_id: number | null;
  tier: Tier;
}

export interface Config {
  lang: "es" | "en";
  steamgrid_key: string;
  steamgrid_enabled: boolean;
  protondb_enabled: boolean;
  gamepad_enabled: boolean;
  scan_steam: boolean;
  scan_epic: boolean;
  scan_heroic: boolean;
  scan_hydra: boolean;
  scan_desktop: boolean;
  watched_folders: string[];
  window_size: [number, number];
}

export interface Prefs {
  sort: Sort;
  source: Source | null;
  platform: Platform | null;
  favorites_only: boolean;
}

export interface Proton {
  name: string;
  path: string;
  version: string;
}

export interface Rgb {
  r: number;
  g: number;
  b: number;
}

export interface LumaGrid {
  width: number;
  height: number;
  values: number[];
}

export interface BackdropInfo {
  url: string;
  luma: LumaGrid;
}

export interface Busy {
  scanning: boolean;
  art: boolean;
  tiers: boolean;
  prefix: boolean;
}

export interface Bootstrap {
  cfg: Config;
  prefs: Prefs;
  games: Game[];
  protons: Proton[];
  data_dir: string;
  media_base: string;
  busy: Busy;
  log: string[];
}

export interface Candidates {
  id: string;
  results: [number, string][];
}
