import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export { invoke, listen, getCurrentWindow };

export function mediaUrl(base, path, width = 0) {
  const q = encodeURIComponent(path);
  return width > 0 ? `${base}/media?path=${q}&w=${width}` : `${base}/media?path=${q}`;
}

export function rgbToHex(r, g, b) {
  return "#" + [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("");
}

export function glowOf(hex) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255;
  let g = (n >> 8) & 255;
  let b = n & 255;
  // Los acentos oscuros sumían el halo en el fondo: se suben de luz conservando el tono.
  const luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
  if (luma < 96) {
    const k = Math.min(3, 96 / Math.max(luma, 24));
    r = Math.round(Math.min(255, r * k + 30));
    g = Math.round(Math.min(255, g * k + 30));
    b = Math.round(Math.min(255, b * k + 30));
  }
  return `rgba(${r}, ${g}, ${b}, 0.65)`;
}

export function accentInk(rgb) {
  const luma = 0.2126 * rgb.r + 0.7152 * rgb.g + 0.0722 * rgb.b;
  if (luma < 120) return "#ffffff";
  if (luma >= 228) return "#000000";
  return "#ffffff";
}
