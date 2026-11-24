# Auera Launcher

Launcher de juegos offline para Windows y Linux, escrito en Rust con egui.

## Compilar

```bash
cargo run            # desarrollo
cargo build --release
```

En Linux necesita `pkg-config`, `libudev` (mandos) y las dependencias de ventana de egui
(`libxkbcommon`, `libwayland`, o las de X11).

## Qué incluye

- Escaneo de bibliotecas: Steam (`libraryfolders.vdf` + `appmanifest_*.acf`, tiempo jugado
  desde `localconfig.vdf`), Epic Games, Heroic (GOG/Epic/Amazon y sideload, nativo y Flatpak,
  incluido `winePrefix`), Hydra (`hydra-db` + `wine-prefixes`) y accesos `.desktop` de Linux.
- Carpetas vigiladas: cada subcarpeta con ejecutable se propone como juego.
- Arte desde SteamGridDB (héroe, portada, logo e icono) cacheado en disco y reutilizado
  entre sesiones; el id resuelto de SteamGridDB también se guarda para no repetir la búsqueda.
- Nivel de compatibilidad de ProtonDB por appid de Steam, con caché local.
- Gestión de Proton/prefijos Wine: detección de versiones (incluidas las del sistema en
  `/usr/share/steam/compatibilitytools.d`), **detección del prefijo de juegos ya existentes**
  (compatdata de Steam en cualquier biblioteca, `wine-prefixes/<id>` de Hydra, `GamesConfig`
  de Heroic —también para juegos de Epic gestionados por Heroic— o un prefijo Wine junto a la
  instalación), **edición manual de la ruta del prefijo** desde la ficha (con re-detección) y
  creación explícita del prefijo cuando falta. Al lanzar se usa siempre el prefijo configurado
  —con `~` expandido— y, si no existe, se avisa en vez de crear otro; la capa Proton sale del
  propio prefijo (`version`/`config_info`) cuando el juego no tiene una elegida.
- Lanzamiento nativo o vía `proton run` con `STEAM_COMPAT_DATA_PATH` y `WINEPREFIX`.
  En Linux los juegos de Windows se lanzan con Proton; el bloqueo manual por sistema
  se marca por juego desde su ficha.
- Interfaz con titlebar propia, rail lateral, biblioteca con portadas y filtros, ficha de
  juego y ajustes; español e inglés.
- Mandos (XInput/DualShock/DualSense) con mapeado fijo para navegación y aceptar/volver.

## Datos locales

- Linux/macOS: `$XDG_CONFIG_HOME/auera` (por defecto `~/.config/auera`).
- Windows: `%APPDATA%\\auera`.

Contiene `config.json`, `library.json`, `protondb.json` y la caché de arte en `art/`.

La clave de SteamGridDB se configura en Ajustes → Integraciones y se guarda solo en
`config.json`. El repositorio incluye una clave de desarrollo como valor inicial.
