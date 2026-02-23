mod imp;

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;

use adw::prelude::*;
use gtk::glib;
use gtk::subclass::prelude::ObjectSubclassIsExt;

glib::wrapper! {
    pub struct NscScreenshotSlot(ObjectSubclass<imp::NscScreenshotSlot>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl NscScreenshotSlot {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn load(&self, url: &str) {
        self.imp().stack.set_visible_child_name("loading");

        let slot_weak = self.downgrade();
        let url = url.to_string();

        glib::spawn_future_local(async move {
            let Some(slot) = slot_weak.upgrade() else {
                return;
            };

            match load_screenshot_pixels(&url).await {
                Ok((pixels, width, height)) => {
                    let stride = width as usize * 4;
                    let gbytes = glib::Bytes::from_owned(pixels);
                    let texture = gtk::gdk::MemoryTexture::new(
                        width,
                        height,
                        gtk::gdk::MemoryFormat::R8g8b8a8,
                        &gbytes,
                        stride,
                    );
                    slot.imp().picture.set_paintable(Some(&texture));
                    slot.imp().stack.set_visible_child_name("image");
                }
                Err(err) => {
                    tracing::warn!("Failed to load screenshot {url}: {err}");
                    slot.imp().stack.set_visible_child_name("error");
                }
            }
        });
    }
}

fn cache_dir() -> PathBuf {
    let mut dir = glib::user_cache_dir();
    dir.push("nix-software-center");
    dir.push("screenshots");
    dir
}

fn cache_path(url: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let hash = hasher.finish();
    cache_dir().join(format!("{hash:016x}"))
}

/// Decoded image data
struct DecodedImage {
    pixels: Vec<u8>,
    width: i32,
    height: i32,
}

/// Decode image bytes into RGBA pixels
/// CPU-intensive, should run off the main thread
fn decode_image_bytes(data: &[u8]) -> Result<DecodedImage, String> {
    let img = image::load_from_memory(data).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok(DecodedImage {
        pixels: rgba.into_raw(),
        width: w as i32,
        height: h as i32,
    })
}

async fn load_screenshot_pixels(
    url: &str,
) -> Result<(Vec<u8>, i32, i32), Box<dyn std::error::Error>> {
    let decoded = crate::runtime::runtime()
        .spawn({
            let url = url.to_string();
            let path = cache_path(&url);

            async move {
                // Try disk cache first
                let data = if let Ok(cached) = tokio::fs::read(&path).await {
                    cached
                } else {
                    let response = reqwest::get(&url).await.map_err(|e| e.to_string())?;
                    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
                    let data = bytes.to_vec();

                    // Write to cache
                    if let Some(parent) = path.parent() {
                        let _ = tokio::fs::create_dir_all(parent).await;
                    }
                    if let Err(err) = tokio::fs::write(&path, &data).await {
                        tracing::warn!("Failed to cache screenshot: {err}");
                    }

                    data
                };

                decode_image_bytes(&data)
            }
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    Ok((decoded.pixels, decoded.width, decoded.height))
}
