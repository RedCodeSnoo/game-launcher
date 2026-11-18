//! Allègement des images du cache (jaquettes, illustrations, logos).
//!
//! Les images d'origine peuvent peser plusieurs mégaoctets (illustrations Epic en 4K, logos PNG de 14 Mo).
//! Sous Windows, chaque image affichée transite par le fil principal de la fenêtre (protocole `asset:` de
//! WebView2) : des centaines de mégaoctets à chaque affichage de la bibliothèque figeaient l'application.
//! Chaque image est donc ramenée à la taille où elle est affichée, une fois pour toutes.
use crate::metadata::ImageKind;
use crate::state::{AppState, Emitter};
use anyhow::Result;
use image::{imageops::FilterType, GenericImageView, ImageEncoder};
use rusqlite::params;
use std::path::Path;

/// Dimensions maximales (largeur, hauteur) : assez pour un écran 4K à la taille d'affichage réelle.
fn max_size(kind: ImageKind) -> (u32, u32) {
    match kind {
        ImageKind::Cover => (600, 900),
        ImageKind::Hero => (1920, 1080),
        ImageKind::Logo => (800, 400),
    }
}

/// En dessous de ce poids, une image aux bonnes dimensions est gardée telle quelle.
fn light_enough(kind: ImageKind) -> usize {
    match kind {
        ImageKind::Cover => 250_000,
        ImageKind::Hero => 500_000,
        ImageKind::Logo => 200_000,
    }
}

/// Version allégée d'une image : `(octets, extension)`, ou `None` si elle est déjà légère ou illisible.
/// Jaquettes et illustrations en JPEG ; logos en PNG (transparence).
pub fn optimize(bytes: &[u8], kind: ImageKind) -> Option<(Vec<u8>, &'static str)> {
    let img = image::load_from_memory(bytes).ok()?;
    let (w, h) = img.dimensions();
    let (mw, mh) = max_size(kind);
    let oversized = w > mw || h > mh;
    if !oversized && bytes.len() <= light_enough(kind) {
        return None;
    }
    let img = if oversized { img.resize(mw, mh, FilterType::CatmullRom) } else { img };
    let mut out = Vec::new();
    let ext = if kind == ImageKind::Logo {
        let rgba = img.to_rgba8();
        image::codecs::png::PngEncoder::new_with_quality(&mut out, image::codecs::png::CompressionType::Best, image::codecs::png::FilterType::Adaptive)
            .write_image(rgba.as_raw(), rgba.width(), rgba.height(), image::ExtendedColorType::Rgba8)
            .ok()?;
        "png"
    } else {
        let rgb = img.to_rgb8();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85).write_image(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8).ok()?;
        "jpg"
    };
    (out.len() < bytes.len()).then_some((out, ext))
}

fn kind_of(file_name: &str) -> Option<ImageKind> {
    // `<id>_<type>_<horodatage>.<ext>`
    let mut parts = file_name.rsplitn(3, '_');
    parts.next()?;
    match parts.next()? {
        "cover" => Some(ImageKind::Cover),
        "hero" => Some(ImageKind::Hero),
        "logo" => Some(ImageKind::Logo),
        _ => None,
    }
}

/// Allège les images déjà en cache (une passe en arrière-plan au démarrage, très courte si tout est déjà fait).
/// Le verrou de la base n'est pris que pour remplacer un chemin.
pub fn optimize_cache(state: &AppState, emit: &Emitter) -> Result<usize> {
    let dir = crate::metadata::images_dir(&state.data_dir);
    let Ok(entries) = std::fs::read_dir(&dir) else { return Ok(0) };
    let mut files: Vec<(std::path::PathBuf, u64)> = entries.flatten().filter_map(|e| Some((e.path(), e.metadata().ok()?.len()))).collect();
    files.sort_by(|a, b| b.1.cmp(&a.1)); // les plus lourdes d'abord : le gain se voit tout de suite
    let mut done = 0;
    for (path, size) in files {
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string()) else { continue };
        let Some(kind) = kind_of(&name) else { continue };
        if (size as usize) <= light_enough(kind) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let Some((small, ext)) = optimize(&bytes, kind) else { continue };
        let Some(new_path) = replace_file(&path, &small, ext) else { continue };
        let (old, new) = (path.to_string_lossy().to_string(), new_path.to_string_lossy().to_string());
        let column = match kind {
            ImageKind::Cover => "cover_path",
            ImageKind::Hero => "hero_path",
            ImageKind::Logo => "logo_path",
        };
        let updated = state.conn().execute(&format!("UPDATE games SET {column} = ?2 WHERE {column} = ?1"), params![old, new])?;
        if updated > 0 {
            let _ = std::fs::remove_file(&path);
        } else {
            // Image qu'aucun jeu n'utilise (ou en cours d'enregistrement) : on n'y touche pas.
            let _ = std::fs::remove_file(&new_path);
            continue;
        }
        done += 1;
        if done % 40 == 0 {
            emit("library-updated", serde_json::Value::Null);
        }
    }
    if done > 0 {
        emit("library-updated", serde_json::Value::Null);
    }
    Ok(done)
}

/// Écrit la version allégée à côté (nouvel horodatage : la vue web ne réutilise pas l'ancienne).
fn replace_file(path: &Path, bytes: &[u8], ext: &str) -> Option<std::path::PathBuf> {
    let stem = path.file_stem()?.to_string_lossy().to_string();
    let base = stem.rsplit_once('_').map(|(b, _)| b.to_string())?;
    let stamp = chrono::Utc::now().timestamp_millis();
    let new_path = path.with_file_name(format!("{base}_{stamp}.{ext}"));
    let tmp = path.with_file_name(format!("{base}_{stamp}.tmp"));
    std::fs::write(&tmp, bytes).ok()?;
    std::fs::rename(&tmp, &new_path).ok()?;
    Some(new_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb, Rgba};

    fn jpeg(w: u32, h: u32) -> Vec<u8> {
        // Dégradé avec un léger motif : se compresse comme une vraie illustration.
        let img = ImageBuffer::from_fn(w, h, |x, y| Rgb([(x * 255 / w) as u8, (y * 255 / h) as u8, (((x / 8 + y / 8) % 2) * 40) as u8]));
        let mut out = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 100).write_image(img.as_raw(), w, h, image::ExtendedColorType::Rgb8).unwrap();
        out
    }

    #[test]
    fn big_images_are_resized_and_small_ones_kept() {
        let big = jpeg(2400, 3600);
        let (small, ext) = optimize(&big, ImageKind::Cover).expect("allégée");
        assert_eq!(ext, "jpg");
        assert!(small.len() < big.len() / 4, "{} -> {}", big.len(), small.len());
        let img = image::load_from_memory(&small).unwrap();
        assert_eq!(img.dimensions(), (600, 900), "proportions gardées");

        let light = jpeg(300, 450);
        assert!(light.len() <= light_enough(ImageKind::Cover));
        assert!(optimize(&light, ImageKind::Cover).is_none(), "déjà légère : inchangée");
        assert!(optimize(b"pas une image", ImageKind::Cover).is_none());
    }

    #[test]
    fn logos_keep_transparency() {
        let img = ImageBuffer::from_fn(3000, 1000, |x, _| Rgba([200, 100, 50, if x % 2 == 0 { 0 } else { 255 }]));
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png).write_image(img.as_raw(), 3000, 1000, image::ExtendedColorType::Rgba8).unwrap();
        let (small, ext) = optimize(&png, ImageKind::Logo).expect("allégé");
        assert_eq!(ext, "png");
        let out = image::load_from_memory(&small).unwrap();
        assert_eq!(out.dimensions(), (800, 267));
        assert!(out.color().has_alpha());
    }

    #[test]
    fn cache_pass_replaces_paths_and_files() {
        let tmp = tempfile::tempdir().unwrap();
        let conn = crate::db::open_in_memory().unwrap();
        let dir = crate::metadata::images_dir(tmp.path());
        std::fs::create_dir_all(&dir).unwrap();
        let big = dir.join("g1_cover_100.jpg");
        std::fs::write(&big, jpeg(2400, 3600)).unwrap();
        let light = dir.join("g1_hero_100.jpg");
        std::fs::write(&light, jpeg(960, 310)).unwrap();
        conn.execute(
            "INSERT INTO games(id, title, sort_title, added_at, cover_path, hero_path) VALUES('g1', 'Jeu', 'jeu', '2025-01-01T00:00:00Z', ?1, ?2)",
            params![big.to_string_lossy(), light.to_string_lossy()],
        )
        .unwrap();
        let state = AppState::new(conn, tmp.path().to_path_buf());
        let emit: Emitter = std::sync::Arc::new(|_: &str, _| {});
        assert_eq!(optimize_cache(&state, &emit).unwrap(), 1);
        let (cover, hero): (String, String) = state.conn().query_row("SELECT cover_path, hero_path FROM games", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_ne!(cover, big.to_string_lossy());
        assert!(Path::new(&cover).is_file() && !big.exists(), "ancienne jaquette remplacée");
        assert!(std::fs::metadata(&cover).unwrap().len() < 250_000);
        assert_eq!(hero, light.to_string_lossy(), "image légère inchangée");
        assert_eq!(optimize_cache(&state, &emit).unwrap(), 0, "deuxième passe : rien à faire");
    }
}
