import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export { invoke, listen, getCurrentWindow };

export function mediaUrl(base, path, width = 0) {
  const q = encodeURIComponent(path);
  return width > 0 ? `${base}/media?path=${q}&w=${width}` : `${base}/media?path=${q}`;
}

// Pide la miniatura antes de pedir la URL: así el protocolo solo lee caché.
const warmed = new Map();

export function warmMedia(path, width) {
  const key = `${path}|${width}`;
  let job = warmed.get(key);
  if (!job) {
    job = invoke("warm_media", { path, width }).catch(() => false);
    warmed.set(key, job);
  }
  return job;
}

export function rgbToHex(r, g, b) {
  return "#" + [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("");
}

// Acento legible sobre el fondo: los tonos oscuros se suben de luz sin cambiar de tono.
export function glowRgb(hex) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255;
  let g = (n >> 8) & 255;
  let b = n & 255;
  const luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
  if (luma < 96) {
    const k = Math.min(3, 96 / Math.max(luma, 24));
    r = Math.round(Math.min(255, r * k + 30));
    g = Math.round(Math.min(255, g * k + 30));
    b = Math.round(Math.min(255, b * k + 30));
  }
  return { r, g, b };
}

export function glowOf(hex) {
  const { r, g, b } = glowRgb(hex);
  return `rgb(${r}, ${g}, ${b})`;
}

export function accentInk(rgb) {
  const luma = 0.2126 * rgb.r + 0.7152 * rgb.g + 0.0722 * rgb.b;
  if (luma < 120) return "#ffffff";
  if (luma >= 228) return "#000000";
  return "#ffffff";
}
