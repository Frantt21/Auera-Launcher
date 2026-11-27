use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

use egui::{ColorImage, Context, TextureHandle, TextureOptions, Vec2};

/// Lado máximo de textura: el arte de SteamGridDB llega hasta 10.000 px y a
/// pantalla completa nunca se necesitan más de ~1600.
const MAX_SIDE: u32 = 1600;
const JPEG_QUALITY: u8 = 85;

#[derive(Clone)]
pub struct Texture {
    pub handle: TextureHandle,
    pub size: Vec2,
    /// Brillo del fondo desenfocado, en una rejilla pequeña. Solo lo llevan los
    /// fondos: sirve para decidir si el cristal debe pasar a negro.
    pub luma: Option<Arc<Luma>>,
}

/// Rejilla de brillo (0..255) de una imagen desenfocada.
#[derive(Clone)]
pub struct Luma {
    width: usize,
    height: usize,
    values: Vec<u8>,
}

impl Luma {
    /// Reduce la imagen a `cells` columnas, conservando su proporción.
    fn from_rgba(image: &image::RgbaImage, cells: usize) -> Self {
        let (width, height) = image.dimensions();
        let columns = cells.max(1).min(width as usize);
        let rows = ((columns as f32 * height as f32 / width as f32).round() as usize)
            .max(1)
            .min(height as usize);
        let data = image.as_raw();
        let mut values = vec![0u8; columns * rows];
        for row in 0..rows {
            let y0 = row * height as usize / rows;
            let y1 = (((row + 1) * height as usize) / rows).max(y0 + 1);
            for column in 0..columns {
                let x0 = column * width as usize / columns;
                let x1 = (((column + 1) * width as usize) / columns).max(x0 + 1);
                let mut sum = 0.0;
                let mut count = 0.0;
                for y in y0..y1 {
                    for x in x0..x1 {
                        let index = (y * width as usize + x) * 4;
                        sum += 0.2126 * data[index] as f32
                            + 0.7152 * data[index + 1] as f32
                            + 0.0722 * data[index + 2] as f32;
                        count += 1.0;
                    }
                }
                values[row * columns + column] = (sum / count).round() as u8;
            }
        }
        Self {
            width: columns,
            height: rows,
            values,
        }
    }

    /// Brillo en coordenadas normalizadas de la textura.
    pub fn sample(&self, u: f32, v: f32) -> u8 {
        let x = (u.clamp(0.0, 1.0) * self.width.saturating_sub(1) as f32).round() as usize;
        let y = (v.clamp(0.0, 1.0) * self.height.saturating_sub(1) as f32).round() as usize;
        self.values[y.min(self.height - 1) * self.width + x.min(self.width - 1)]
    }
}

type RawImage = (ColorImage, Vec2, Luma);
/// Imagen decodificada, su tamaño y, para los fondos, la copia desenfocada.
type LoadedImage = (ColorImage, Vec2, Option<RawImage>);

/// Carga de imágenes en hilos aparte para no bloquear el repintado.
pub struct Textures {
    cache: HashMap<PathBuf, Option<Texture>>,
    blurred: HashMap<PathBuf, Option<Texture>>,
    requested: HashSet<PathBuf>,
    jobs: Sender<PathBuf>,
    urgent: Sender<PathBuf>,
    loaded: Receiver<(PathBuf, Option<LoadedImage>)>,
}

impl Default for Textures {
    fn default() -> Self {
        Self::new()
    }
}

impl Textures {
    pub fn new() -> Self {
        let (jobs, pending) = mpsc::channel::<PathBuf>();
        let (urgent, priority) = mpsc::channel::<PathBuf>();
        let (done, loaded) = mpsc::channel();
        let (thumbs, thumb_queue) = mpsc::channel();
        spawn_loader(pending, done.clone(), thumbs.clone());
        spawn_loader(priority, done.clone(), thumbs.clone());
        spawn_thumb_writer(thumb_queue);
        Self {
            cache: HashMap::new(),
            blurred: HashMap::new(),
            requested: HashSet::new(),
            jobs,
            urgent,
            loaded,
        }
    }

    /// Devuelve la textura si ya está lista; si no, encola su carga.
    pub fn get(&mut self, path: Option<&Path>) -> Option<Texture> {
        self.fetch(path, false)
    }

    /// Igual que `get`, pero en el hilo prioritario: el fondo y el logo de la
    /// ficha no deben esperar detrás de las portadas de la biblioteca.
    pub fn get_urgent(&mut self, path: Option<&Path>) -> Option<Texture> {
        self.fetch(path, true)
    }

    fn fetch(&mut self, path: Option<&Path>, urgent: bool) -> Option<Texture> {
        let path = path?;
        if let Some(cached) = self.cache.get(path) {
            return cached.clone();
        }
        if self.requested.insert(path.to_path_buf()) {
            let sender = if urgent { &self.urgent } else { &self.jobs };
            let _ = sender.send(path.to_path_buf());
        }
        None
    }

    /// Copia desenfocada de un fondo, ya disponible si su imagen se cargó.
    pub fn get_blurred(&mut self, path: Option<&Path>) -> Option<Texture> {
        let path = path?;
        self.blurred.get(path).cloned().flatten()
    }

    pub fn forget(&mut self, path: &Path) {
        self.cache.remove(path);
        self.blurred.remove(path);
        self.requested.remove(path);
    }

    pub fn is_busy(&self) -> bool {
        !self.requested.is_empty()
    }

    /// Sube a GPU las imágenes ya decodificadas, en tandas cortas: volcar
    /// muchas texturas en un mismo fotograma provoca tirones al arrancar.
    pub fn poll(&mut self, ctx: &Context) -> bool {
        let mut updated = false;
        for _ in 0..2 {
            let Ok((path, image)) = self.loaded.try_recv() else {
                break;
            };
            self.requested.remove(&path);
            let texture = image.map(|(image, size, blurred)| {
                let handle =
                    ctx.load_texture(path.to_string_lossy(), image, TextureOptions::LINEAR);
                let blurred = blurred.map(|(image, size, luma)| Texture {
                    handle: ctx.load_texture(
                        format!("{}·blur", path.to_string_lossy()),
                        image,
                        TextureOptions::LINEAR,
                    ),
                    size,
                    luma: Some(Arc::new(luma)),
                });
                (
                    Texture {
                        handle,
                        size,
                        luma: None,
                    },
                    blurred,
                )
            });
            match texture {
                Some((texture, blurred)) => {
                    self.blurred.insert(path.clone(), blurred);
                    self.cache.insert(path, Some(texture));
                }
                None => {
                    self.blurred.insert(path.clone(), None);
                    self.cache.insert(path, None);
                }
            }
            updated = true;
        }
        updated
    }
}

fn spawn_loader(
    pending: Receiver<PathBuf>,
    done: Sender<(PathBuf, Option<LoadedImage>)>,
    thumbs: Sender<(PathBuf, image::DynamicImage)>,
) {
    std::thread::spawn(move || {
        while let Ok(path) = pending.recv() {
            let image = load(&path, &thumbs);
            if done.send((path, image)).is_err() {
                break;
            }
        }
    });
}

/// La copia reducida se escribe en otro hilo: así el logo o el fondo aparecen
/// en pantalla sin esperar a codificar el PNG/JPEG.
fn spawn_thumb_writer(queue: Receiver<(PathBuf, image::DynamicImage)>) {
    std::thread::spawn(move || {
        while let Ok((path, image)) = queue.recv() {
            write_thumb(&path, &image);
        }
    });
}

/// Decodifica una imagen. El arte que supera `MAX_SIDE` se reduce y se guarda
/// en disco: decodificar PNG de varios megapíxeles en cada arranque es lo que
/// hacía lentos los logos grandes y los fondos.
fn load(path: &Path, thumbs: &Sender<(PathBuf, image::DynamicImage)>) -> Option<LoadedImage> {
    let cacheable = !path.starts_with(crate::config::data_dir());
    if cacheable {
        if let Some(thumb) = existing_thumb(path) {
            if let Some((image, size)) = read(&thumb) {
                return Some((image, size, None));
            }
        }
    }

    let source = image::open(path).ok()?;
    let oversized = source.width() > MAX_SIDE || source.height() > MAX_SIDE;
    let image = if oversized {
        source.thumbnail(MAX_SIDE, MAX_SIDE)
    } else {
        source
    };
    let (pixels, size) = colorize(&image);
    // Solo los fondos necesitan el cristal: generar el desenfoque del resto de
    // imágenes (portadas, logos) sería trabajo tirado.
    let blurred = is_backdrop(path).then(|| blur(&image));
    if oversized && cacheable {
        if let Some(thumb) = thumb_path(path, image.color().has_alpha()) {
            let _ = thumbs.send((thumb, image));
        }
    }
    Some((pixels, size, blurred))
}

fn is_backdrop(path: &Path) -> bool {
    path.starts_with(crate::config::data_dir().join("backdrops"))
}

/// Copia muy pequeña y desenfocada: se reutiliza para el cristal esmerilado.
fn blur(image: &image::DynamicImage) -> RawImage {
    let small = image.thumbnail(512, 512).to_rgba8();
    let blurred = image::imageops::blur(&small, 3.0);
    let size = Vec2::new(blurred.width() as f32, blurred.height() as f32);
    let pixels = ColorImage::from_rgba_unmultiplied(
        [blurred.width() as usize, blurred.height() as usize],
        blurred.as_raw(),
    );
    let luma = Luma::from_rgba(&blurred, 32);
    (pixels, size, luma)
}

fn read(path: &Path) -> Option<(ColorImage, Vec2)> {
    let image = image::open(path).ok()?;
    Some(colorize(&image))
}

fn colorize(image: &image::DynamicImage) -> (ColorImage, Vec2) {
    let rgba = image.to_rgba8();
    let size = Vec2::new(rgba.width() as f32, rgba.height() as f32);
    let pixels = ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    );
    (pixels, size)
}

/// Copia reducida ya escrita, si existe. Lleva la fecha del original en el
/// nombre, así que al actualizar el arte se vuelve a generar.
fn existing_thumb(source: &Path) -> Option<PathBuf> {
    for alpha in [true, false] {
        let path = thumb_path(source, alpha)?;
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn thumb_path(source: &Path, alpha: bool) -> Option<PathBuf> {
    let modified = std::fs::metadata(source).ok()?.modified().ok()?;
    let stamp = modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos() as u64;
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in source.to_string_lossy().bytes().chain(stamp.to_le_bytes()) {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    let folder = crate::config::data_dir().join("textures");
    Some(folder.join(format!("{hash:016x}.{}", if alpha { "png" } else { "jpg" })))
}

fn write_thumb(path: &Path, image: &image::DynamicImage) {
    let Some(folder) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(folder).is_err() {
        return;
    }
    let temp = path.with_extension("tmp");
    let saved = if image.color().has_alpha() {
        image.save_with_format(&temp, image::ImageFormat::Png)
    } else {
        let file = match std::fs::File::create(&temp) {
            Ok(file) => file,
            Err(_) => return,
        };
        let mut writer = std::io::BufWriter::new(file);
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, JPEG_QUALITY)
            .encode_image(image)
            .and_then(|()| std::io::Write::flush(&mut writer).map_err(image::ImageError::IoError))
    };
    if saved.is_ok() {
        let _ = std::fs::rename(&temp, path);
    } else {
        let _ = std::fs::remove_file(&temp);
    }
}
