use adw::prelude::*;
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

/// Load an icon from an AppStream component into a `GtkImage`.
pub fn load_component_icon(image: &gtk::Image, component: &libappstream::Component, sizes: &[u32]) {
    for &size in sizes {
        if let Some(icon) = component.icon_by_size(size, size) {
            match IconExt::kind(&icon) {
                libappstream::IconKind::Cached => {
                    if let Some(filename) = icon.filename() {
                        let path = std::path::Path::new(filename.as_str());
                        if path.exists() {
                            image.set_from_file(Some(filename.as_str()));
                            return;
                        }
                    }
                }
                libappstream::IconKind::Stock => {
                    if let Some(name) = IconExt::name(&icon) {
                        image.set_icon_name(Some(name.as_str()));
                        return;
                    }
                }
                _ => {}
            }
        }
    }

    // Fall back to stock icon or generic
    if let Some(icon) = component.icon_stock()
        && let Some(name) = IconExt::name(&icon)
    {
        image.set_icon_name(Some(name.as_str()));
        return;
    }
    image.set_icon_name(Some("application-x-executable"));
}

/// Check if a component has a `.desktop` file installed on the system
pub fn has_system_desktop_file(component: &libappstream::Component) -> bool {
    let desktop_id = component
        .launchable(libappstream::LaunchableKind::DesktopId)
        .and_then(|l| l.entries().into_iter().next())
        .or_else(|| component.id());

    desktop_id.is_some_and(|id| gio_unix::DesktopAppInfo::new(&id).is_some())
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
