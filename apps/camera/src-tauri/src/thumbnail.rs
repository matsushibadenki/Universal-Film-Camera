//! Disposable, bounded ImageIO previews. Originals are never modified.
use objc2_core_foundation::{
    CFBoolean, CFDictionary, CFMutableData, CFNumber, CFString, CFType, CFURL,
};
use objc2_image_io::*;
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path, sync::Mutex, time::UNIX_EPOCH};

static DECODE_LOCK: Mutex<()> = Mutex::new(());
const EDGE: isize = 512;
const BUDGET: u64 = 64 * 1024 * 1024;

fn fingerprint(path: &Path) -> Result<String, String> {
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() == 0 || meta.len() > 64 * 1024 * 1024 {
        return Err("invalid thumbnail source size".into());
    }
    let modified = meta
        .modified()
        .map_err(|e| e.to_string())?
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let value = format!(
        "imageio-v1-{EDGE}:{}:{}:{modified}",
        path.display(),
        meta.len()
    );
    Ok(format!("{:x}", Sha256::digest(value.as_bytes())))
}

fn decode(path: &Path) -> Result<Vec<u8>, String> {
    let url = CFURL::from_file_path(path).ok_or("invalid image URL")?;
    // ImageIO owns its decoded objects; retained CF values outlive each call.
    // Only documented ImageIO option keys and corresponding CF types are used.
    unsafe {
        let options = CFDictionary::<CFType, CFType>::from_slices(
            &[
                kCGImageSourceCreateThumbnailFromImageAlways.as_ref(),
                kCGImageSourceCreateThumbnailWithTransform.as_ref(),
                kCGImageSourceThumbnailMaxPixelSize.as_ref(),
                kCGImageSourceShouldCache.as_ref(),
            ],
            &[
                CFBoolean::new(true).as_ref(),
                CFBoolean::new(true).as_ref(),
                CFNumber::new_isize(EDGE).as_ref(),
                CFBoolean::new(false).as_ref(),
            ],
        );
        let source = CGImageSource::with_url(&url, Some(options.as_ref()))
            .ok_or("ImageIO could not open image")?;
        let image = source
            .thumbnail_at_index(0, Some(options.as_ref()))
            .ok_or("ImageIO could not downsample image")?;
        let data = CFMutableData::new(None, 0).ok_or("thumbnail allocation failed")?;
        let destination =
            CGImageDestination::with_data(&data, &CFString::from_str("public.jpeg"), 1, None)
                .ok_or("thumbnail encoder unavailable")?;
        destination.add_image(&image, None);
        if !destination.finalize() {
            return Err("thumbnail encoding failed".into());
        }
        Ok(data.to_vec())
    }
}

fn prune(directory: &Path, reserve: u64) -> Result<(), String> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().map_err(|e| e.to_string())?.is_file() {
            continue;
        }
        if entry.path().extension().is_none_or(|ext| ext != "jpg") {
            continue;
        }
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        entries.push((
            meta.modified().unwrap_or(UNIX_EPOCH),
            meta.len(),
            entry.path(),
        ));
    }
    entries.sort_by_key(|entry| entry.0);
    let mut total = entries.iter().map(|entry| entry.1).sum::<u64>();
    for (_, size, path) in entries {
        if total.saturating_add(reserve) <= BUDGET {
            break;
        }
        fs::remove_file(path).map_err(|e| e.to_string())?;
        total = total.saturating_sub(size);
    }
    Ok(())
}

pub fn load(source: &Path, directory: &Path) -> Result<Vec<u8>, String> {
    // At most one large source is decoded at once, even when all cards request IPC.
    let _guard = DECODE_LOCK.lock().map_err(|_| "thumbnail lock poisoned")?;
    let key = fingerprint(source)?;
    // A full/unavailable cache must not make otherwise readable photos disappear.
    let _ = fs::create_dir_all(directory);
    let target = directory.join(format!("{key}.jpg"));
    if let Ok(meta) = fs::symlink_metadata(&target) {
        if meta.file_type().is_file() && meta.len() <= 2 * 1024 * 1024 {
            if let Ok(bytes) = fs::read(&target) {
                if bytes.starts_with(&[0xff, 0xd8]) && bytes.ends_with(&[0xff, 0xd9]) {
                    return Ok(bytes);
                }
            }
        }
    }
    let bytes = decode(source)?;
    if fingerprint(source)? != key {
        return Err("image changed while thumbnail was generated".into());
    }
    // Cache failures must not prevent showing a successfully decoded image.
    let _ = (|| -> Result<(), String> {
        prune(directory, bytes.len() as u64)?;
        let temporary = directory.join(format!("{key}.partial"));
        // Serialized writes; remove only our known incomplete cache entry.
        if temporary.exists() {
            fs::remove_file(&temporary).map_err(|e| e.to_string())?;
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        fs::rename(&temporary, &target).map_err(|e| e.to_string())?;
        Ok(())
    })();
    Ok(bytes)
}

/// Purge obsolete versions and deleted assets when the media catalogue refreshes.
pub fn retain_sources(sources: &[std::path::PathBuf], directory: &Path) -> Result<(), String> {
    let _guard = DECODE_LOCK.lock().map_err(|_| "thumbnail lock poisoned")?;
    if !directory.exists() {
        return Ok(());
    }
    let live = sources
        .iter()
        .map(|source| fingerprint(source).map(|key| format!("{key}.jpg")))
        .collect::<Result<std::collections::HashSet<_>, _>>()?;
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().map_err(|e| e.to_string())?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if (name.ends_with(".jpg") && !live.contains(&name)) || name.ends_with(".partial") {
            fs::remove_file(entry.path()).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exif_orientation_is_baked_into_thumbnail_dimensions() {
        // Minimal uncompressed RGB TIFF: 2×1 pixels with orientation 6 (90°).
        // Construct the fixture locally so this test needs neither tools nor a camera.
        let root =
            std::env::temp_dir().join(format!("ufc-thumbnail-orientation-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("rotated.tiff");
        let mut tiff = b"II\x2a\x00\x08\x00\x00\x00".to_vec();
        tiff.extend_from_slice(&10u16.to_le_bytes());
        for (tag, kind, count, value) in [
            (256u16, 4u16, 1u32, 2u32), // width
            (257, 4, 1, 1),             // height
            (258, 3, 3, 134),           // RGB bits at offset 134
            (259, 3, 1, 1),             // no compression
            (262, 3, 1, 2),             // RGB
            (273, 4, 1, 140),           // strip offset
            (274, 3, 1, 6),             // orientation
            (277, 3, 1, 3),             // samples/pixel
            (278, 4, 1, 1),             // rows/strip
            (279, 4, 1, 6),             // strip byte count
        ] {
            tiff.extend_from_slice(&tag.to_le_bytes());
            tiff.extend_from_slice(&kind.to_le_bytes());
            tiff.extend_from_slice(&count.to_le_bytes());
            tiff.extend_from_slice(&value.to_le_bytes());
        }
        tiff.extend_from_slice(&0u32.to_le_bytes()); // next IFD
        tiff.extend_from_slice(&[8, 0, 8, 0, 8, 0, 255, 0, 0, 0, 0, 255]);
        fs::write(&source, tiff).unwrap();
        let output = root.join("preview.jpg");
        fs::write(&output, load(&source, &root.join("cache")).unwrap()).unwrap();
        let probe =
            camera_core::probe_media_resource(&output, camera_core::CapturedMediaType::Photo)
                .unwrap();
        assert_eq!((probe.pixel_width, probe.pixel_height), (1, 2));
        assert_eq!(probe.orientation, 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_thumbnail_is_small_cached_and_invalidated() {
        let root = std::env::temp_dir().join(format!("ufc-thumbnail-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.png");
        fs::write(
            &source,
            include_bytes!("../../../../assets/design/app-icon-concept.png"),
        )
        .unwrap();
        let cache = root.join("cache");
        let first = load(&source, &cache).unwrap();
        assert!(first.starts_with(&[0xff, 0xd8]));
        assert!(first.len() < 2 * 1024 * 1024);
        let output = root.join("preview.jpg");
        fs::write(&output, &first).unwrap();
        let probe =
            camera_core::probe_media_resource(&output, camera_core::CapturedMediaType::Photo)
                .unwrap();
        assert_eq!(probe.pixel_width.max(probe.pixel_height), EDGE as u32);
        assert_eq!(probe.orientation, 1);
        assert_eq!(first, load(&source, &cache).unwrap());
        // A failed cache directory still permits an image to be displayed.
        let unavailable = root.join("not-a-directory");
        fs::write(&unavailable, b"occupied").unwrap();
        assert_eq!(first, load(&source, &unavailable).unwrap());
        // Incomplete cached bytes are regenerated.
        fs::write(
            cache.join(format!("{}.jpg", fingerprint(&source).unwrap())),
            b"broken",
        )
        .unwrap();
        assert_eq!(first, load(&source, &cache).unwrap());
        // Sparse oversized cache data exercises eviction without allocating 64 MiB.
        let oversized = cache.join("obsolete.jpg");
        fs::File::create(&oversized)
            .unwrap()
            .set_len(BUDGET + 1)
            .unwrap();
        prune(&cache, 0).unwrap();
        assert!(!oversized.exists());
        let old_key = fingerprint(&source).unwrap();
        fs::write(&source, include_bytes!("../icons/32x32.png")).unwrap();
        assert_ne!(old_key, fingerprint(&source).unwrap());
        assert_ne!(first, load(&source, &cache).unwrap());
        retain_sources(&[source.clone()], &cache).unwrap();
        assert_eq!(fs::read_dir(&cache).unwrap().count(), 1);
        retain_sources(&[], &cache).unwrap();
        assert_eq!(fs::read_dir(&cache).unwrap().count(), 0);
        fs::write(&source, b"not an image").unwrap();
        assert!(load(&source, &cache).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
