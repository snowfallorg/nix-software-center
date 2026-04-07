use std::path::PathBuf;

use adw::prelude::*;
use gtk::{gio, glib};
use libappstream::prelude::{ComponentExt, IconExt, LaunchableExt};

/// Walk up the widget tree from `widget` to find the nearest `NavigationView`.
pub fn find_navigation_view(widget: &impl IsA<gtk::Widget>) -> Option<adw::NavigationView> {
    let mut ancestor = widget.parent();
    while let Some(w) = ancestor {
        if let Ok(nav) = w.clone().downcast::<adw::NavigationView>() {
            return Some(nav);
        }
        ancestor = w.parent();
    }
    None
}

/// Resolved icon source
pub enum IconSource {
    File { path: PathBuf, fallback: String },
    Stock(String),
}

/// Resolve an AppStream component's icon to a path or stock name
pub fn resolve_component_icon(component: &libappstream::Component, sizes: &[u32]) -> IconSource {
    let mut cached_path = None;

    for &size in sizes {
        if let Some(icon) = component.icon_by_size(size, size) {
            match IconExt::kind(&icon) {
                libappstream::IconKind::Cached => {
                    if cached_path.is_none()
                        && let Some(filename) = icon.filename()
                    {
                        cached_path = Some(PathBuf::from(filename.as_str()));
                    }
                }
                libappstream::IconKind::Stock => {
                    if let Some(name) = IconExt::name(&icon) {
                        return IconSource::Stock(name.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    let fallback = component
        .icon_stock()
        .and_then(|icon| IconExt::name(&icon).map(|n| n.to_string()))
        .unwrap_or_else(|| "application-x-executable".into());

    match cached_path {
        Some(path) => IconSource::File { path, fallback },
        None => IconSource::Stock(fallback),
    }
}

/// Load a file-based icon asynchronously, showing stock fallback immediately
pub fn load_icon_async(
    image: &gtk::Image,
    source: IconSource,
    generation: u64,
    current_generation: impl Fn() -> u64 + 'static,
) {
    match source {
        IconSource::Stock(name) => {
            image.set_icon_name(Some(&name));
        }
        IconSource::File { path, fallback } => {
            image.set_icon_name(Some(&fallback));

            let image = image.clone();
            glib::spawn_future_local(async move {
                let texture = gio::spawn_blocking(move || {
                    let data = std::fs::read(&path).ok()?;
                    let bytes = glib::Bytes::from_owned(data);
                    gtk::gdk::Texture::from_bytes(&bytes).ok()
                })
                .await
                .ok()
                .flatten();

                if current_generation() != generation {
                    return;
                }

                if let Some(texture) = texture {
                    image.set_paintable(Some(&texture));
                }
            });
        }
    }
}

/// Check if a component has a `.desktop` file installed on the system
pub fn has_system_desktop_file(
    component: &libappstream::Component,
    desktop_ids: &std::collections::HashSet<String>,
) -> bool {
    let desktop_id = component
        .launchable(libappstream::LaunchableKind::DesktopId)
        .and_then(|l| l.entries().into_iter().next())
        .or_else(|| component.id());

    desktop_id.is_some_and(|id| desktop_ids.contains(id.as_str()))
}

/// Strip a nix output suffix from a package attribute if present
pub fn strip_nix_output_suffix(attr: &str) -> &str {
    if let Some((base, suffix)) = attr.rsplit_once('.') {
        match suffix {
            "out" | "dev" | "devdoc" | "lib" | "man" | "doc" | "info" | "bin" | "debug"
            | "static" | "all" => base,
            _ => attr,
        }
    } else {
        attr
    }
}
