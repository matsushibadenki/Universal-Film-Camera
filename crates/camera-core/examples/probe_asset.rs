use camera_core::{CapturedMediaType, probe_media_resource};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let media_type = match arguments.next().as_deref() {
        Some("photo") => CapturedMediaType::Photo,
        Some("video") => CapturedMediaType::Video,
        _ => return Err("usage: probe_asset <photo|video> <path>".into()),
    };
    let path = arguments
        .next()
        .ok_or("usage: probe_asset <photo|video> <path>")?;
    if arguments.next().is_some() {
        return Err("usage: probe_asset <photo|video> <path>".into());
    }
    println!("{:#?}", probe_media_resource(path, media_type)?);
    Ok(())
}
