import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export { invoke, listen, getCurrentWindow };

export function mediaUrl(base: string, path: string, width = 0): string {
  const q = encodeURIComponent(path);
  return width > 0 ? `${base}/media?path=${q}&w=${width}` : `${base}/media?path=${q}`;
}

export function rgbToHex(r: number, g: number, b: number): string {
  return "#" + [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("");
}

export function accentInk(rgb: { r: number; g: number; b: number }): string {
  const luma = 0.2126 * rgb.r + 0.7152 * rgb.g + 0.0722 * rgb.b;
  if (luma < 120) return "#ffffff";
  if (luma >= 228) return "#000000";
  return "#ffffff";
}

export function lumaOf(rgb: { r: number; g: number; b: number }): number {
  return 0.2126 * rgb.r + 0.7152 * rgb.g + 0.0722 * rgb.b;
}
