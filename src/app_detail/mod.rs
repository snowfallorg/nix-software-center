mod imp;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use libappstream::prelude::*;

glib::wrapper! {
    pub struct NscAppDetail(ObjectSubclass<imp::NscAppDetail>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl NscAppDetail {
    pub fn new(
        component: &libappstream::Component,
        metadata: &libsnow::metadata::Metadata,
    ) -> Self {
        let page: Self = glib::Object::new();
        page.populate(component, metadata);
        page
    }

    fn populate(
        &self,
        component: &libappstream::Component,
        metadata: &libsnow::metadata::Metadata,
    ) {
        let imp = self.imp();

        let pkg_info = component
            .pkgname()
            .and_then(|pkgname| metadata.get(pkgname.as_str()).ok());

        if let Some(name) = component.name() {
            self.set_title(name.as_str());
            imp.name_label.set_label(name.as_str());
        }

        if let Some(summary) = component.summary() {
            imp.summary_label.set_label(summary.as_str());
        }

        // Developer
        let developer_name = extract_developer_name(component)
            .or_else(|| component.project_group().map(|g| g.to_string()));
        if let Some(name) = developer_name {
            imp.developer_label.set_label(&name);
            imp.developer_label.set_visible(true);
        } else {
            imp.developer_label.set_visible(false);
        }

        // Icon
        Self::load_icon(imp, component);

        // Description
        if let Some(desc) = component.description() {
            let buffer = imp.description_view.buffer();
            populate_description_buffer(&buffer, &desc);
            imp.description_section.set_visible(true);
            // Load text view content before the clamp measures
            let clamp_weak = imp.description_clamp.downgrade();
            glib::idle_add_local_full(glib::Priority::LOW, move || {
                if let Some(clamp) = clamp_weak.upgrade() {
                    clamp.queue_resize();
                }
                glib::ControlFlow::Break
            });
        }

        if let Some(ref pi) = pkg_info {
            imp.version_label.set_label(&pi.version);
            imp.version_row.set_visible(true);
        }

        // License
        if let Some(license) = component.project_license() {
            imp.license_label.set_label(license.as_str());
            imp.license_row.set_visible(true);

            let license_data: Vec<(String, String)> = collect_spdx_license_ids(&license)
                .into_iter()
                .map(|id| (id.full_name.to_string(), id.text().to_string()))
                .collect();

            if !license_data.is_empty() {
                imp.license_row.set_activatable(true);
                imp.license_row.connect_activated(move |row| {
                    show_license_dialog(row, &license_data);
                });
            }
        }

        // Package name (nix attribute)
        if let Some(ref pi) = pkg_info {
            imp.package_label.set_label(&pi.attribute);
            imp.package_row.set_visible(true);
        } else if let Some(pkgname) = component.pkgname() {
            imp.package_label.set_label(pkgname.as_str());
            imp.package_row.set_visible(true);
        }

        // Support button (donate link)
        if let Some(url) = component.url(libappstream::UrlKind::Donation) {
            let url_str = url.to_string();
            imp.support_button.set_visible(true);
            imp.support_button.connect_clicked(move |btn| {
                open_url(btn, &url_str);
            });
        }

        // Screenshots
        self.populate_screenshots(component);

        // Links
        self.populate_links(component);
    }

    fn populate_screenshots(&self, component: &libappstream::Component) {
        let screenshots = component.screenshots_all();

        if screenshots.is_empty() {
            return;
        }

        // Collect URLs to load.
        let mut urls = Vec::new();
        for screenshot in &screenshots {
            let image = screenshot
                .images()
                .into_iter()
                .find(|img| img.kind() == libappstream::ImageKind::Source)
                .or_else(|| {
                    let mut imgs = screenshot.images();
                    imgs.sort_by(|a, b| {
                        let area_a = a.width() as u64 * a.height() as u64;
                        let area_b = b.width() as u64 * b.height() as u64;
                        area_b.cmp(&area_a)
                    });
                    imgs.into_iter().next()
                });

            if let Some(image) = image
                && let Some(url) = image.url()
            {
                urls.push(url.to_string());
            }
        }

        if urls.is_empty() {
            return;
        }

        let imp = self.imp();

        // Show the section immediately with a loading spinner per screenshot.
        let urls_count = urls.len();
        imp.screenshot_box.set_visible(true);
        if urls_count > 1 {
            imp.screenshot_dots.set_visible(true);
        } else {
            imp.screenshot_dots.set_visible(false);
            imp.screenshot_carousel
                .set_margin_bottom(imp.screenshot_carousel.margin_top());
        }

        for url in urls {
            let slot = crate::screenshot_slot::NscScreenshotSlot::new();
            imp.screenshot_carousel.append(&slot);
            slot.load(&url);
        }

        // Set initial nav button state (position_notify doesn't fire on first load).
        if urls_count > 1 {
            imp.screenshot_next_revealer.set_reveal_child(true);
            imp.screenshot_next_revealer.set_can_target(true);
        }
    }

    fn populate_links(&self, component: &libappstream::Component) {
        let imp = self.imp();
        let mut has_links = false;

        let setup_link_row = |row: &adw::ActionRow, url: glib::GString| {
            let url_str = url.to_string();
            row.set_subtitle(&url_str);
            row.set_visible(true);
            row.connect_activated(move |row| {
                open_url(row, &url_str);
            });
        };

        if let Some(url) = component.url(libappstream::UrlKind::Homepage) {
            setup_link_row(&imp.homepage_row, url);
            has_links = true;
        }

        if let Some(url) = component.url(libappstream::UrlKind::Bugtracker) {
            setup_link_row(&imp.bugtracker_row, url);
            has_links = true;
        }

        if let Some(url) = component.url(libappstream::UrlKind::Help) {
            setup_link_row(&imp.help_row, url);
            has_links = true;
        }

        if let Some(url) = component.url(libappstream::UrlKind::Donation) {
            setup_link_row(&imp.donate_row, url);
            has_links = true;
        }

        imp.links_group.set_visible(has_links);
    }

    fn load_icon(imp: &imp::NscAppDetail, component: &libappstream::Component) {
        crate::util::load_component_icon(&imp.icon, component, &[128, 64, 48]);
    }
}

/// Extract the developer name from a component's XML data
fn extract_developer_name(component: &libappstream::Component) -> Option<String> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let ctx = libappstream::Context::new();
    let xml = component.to_xml_data(&ctx).ok()?;

    let mut reader = Reader::from_str(xml.as_str());
    let mut in_developer = false;
    let mut in_developer_name_legacy = false;
    let mut depth = 0u32;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let qname = e.name();
                let name = std::str::from_utf8(qname.as_ref()).unwrap_or("");
                match name {
                    "developer" => {
                        in_developer = true;
                        depth = 0;
                    }
                    "developer_name" => {
                        in_developer_name_legacy = true;
                    }
                    "name" if in_developer && depth == 0 => {
                        depth = 1;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                if (in_developer && depth == 1) || in_developer_name_legacy {
                    let decoded = e.decode().unwrap_or_default();
                    let text = decoded.trim().to_string();
                    if !text.is_empty() {
                        return Some(text);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let qname = e.name();
                let name = std::str::from_utf8(qname.as_ref()).unwrap_or("");
                match name {
                    "developer" => {
                        in_developer = false;
                    }
                    "developer_name" => {
                        in_developer_name_legacy = false;
                    }
                    "name" if in_developer => {
                        depth = 0;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    None
}

fn populate_description_buffer(buffer: &gtk::TextBuffer, xml: &str) {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let wrapped = format!("<root>{xml}</root>");
    let mut reader = Reader::from_str(&wrapped);

    let mut tag_stack: Vec<&str> = Vec::new();
    let mut list_ctx: Vec<(bool, u32)> = Vec::new();
    let mut need_paragraph_break = false;
    let mut li_start_offset: Option<i32> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let qname = e.name();
                let name = std::str::from_utf8(qname.as_ref()).unwrap_or("");
                match name {
                    "p" => {
                        if need_paragraph_break {
                            let mut end = buffer.end_iter();
                            buffer.insert(&mut end, "\n");
                        }
                        tag_stack.push("paragraph");
                    }
                    "ul" => {
                        if need_paragraph_break {
                            let mut end = buffer.end_iter();
                            buffer.insert(&mut end, "\n");
                        }
                        list_ctx.push((false, 0));
                    }
                    "ol" => {
                        if need_paragraph_break {
                            let mut end = buffer.end_iter();
                            buffer.insert(&mut end, "\n");
                        }
                        list_ctx.push((true, 0));
                    }
                    "li" => {
                        if let Some((ordered, counter)) = list_ctx.last_mut() {
                            *counter += 1;
                            if *counter > 1 {
                                let mut end = buffer.end_iter();
                                buffer.insert(&mut end, "\n");
                            }
                            // Record where this list item starts (including prefix)
                            li_start_offset = Some(buffer.end_iter().offset());
                            let prefix = if *ordered {
                                format!("{}. ", *counter)
                            } else {
                                "\u{2022} ".to_string()
                            };
                            let mut end = buffer.end_iter();
                            buffer.insert(&mut end, &prefix);
                        }
                    }
                    "em" => {
                        tag_stack.push("bold");
                    }
                    "code" => {
                        tag_stack.push("monospace");
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let qname = e.name();
                let name = std::str::from_utf8(qname.as_ref()).unwrap_or("");
                match name {
                    "p" => {
                        tag_stack.pop();
                        need_paragraph_break = true;
                    }
                    "ul" | "ol" => {
                        list_ctx.pop();
                        need_paragraph_break = true;
                    }
                    "li" => {
                        if let Some(start_off) = li_start_offset.take() {
                            let start = buffer.iter_at_offset(start_off);
                            let end = buffer.end_iter();
                            buffer.apply_tag_by_name("list-item", &start, &end);
                        }
                    }
                    "em" | "code" => {
                        tag_stack.pop();
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                let decoded = e.decode().unwrap_or_default();
                let unescaped =
                    quick_xml::escape::unescape(&decoded).unwrap_or_else(|_| decoded.clone());
                let text: String = unescaped.split_whitespace().collect::<Vec<_>>().join(" ");
                if text.is_empty() {
                    continue;
                }

                let start_offset = buffer.end_iter().offset();
                let mut end = buffer.end_iter();
                buffer.insert(&mut end, &text);

                let start = buffer.iter_at_offset(start_offset);
                let end = buffer.end_iter();
                for tag_name in &tag_stack {
                    buffer.apply_tag_by_name(tag_name, &start, &end);
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                tracing::warn!("Error parsing AppStream description XML: {}", err);
                break;
            }
            _ => {}
        }
    }
}

fn collect_spdx_license_ids(expression: &str) -> Vec<spdx::LicenseId> {
    let mut ids = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for token in expression.split_whitespace() {
        let cleaned = token.trim_matches(|c| c == '(' || c == ')');
        let resolved = if cleaned.ends_with('+') {
            let base = cleaned.trim_end_matches('+');
            let or_later = format!("{base}-or-later");
            spdx::license_id(&or_later).or_else(|| spdx::license_id(base))
        } else {
            spdx::license_id(cleaned)
        };

        if let Some(id) = resolved
            && seen.insert(id.full_name)
        {
            ids.push(id);
        }
    }
    ids
}

fn build_license_text_page(name: &str, text: &str) -> adw::NavigationPage {
    let view = gtk::TextView::new();
    view.set_editable(false);
    view.set_cursor_visible(false);
    view.set_wrap_mode(gtk::WrapMode::Word);
    view.set_top_margin(12);
    view.set_bottom_margin(12);
    view.set_left_margin(12);
    view.set_right_margin(12);
    view.add_css_class("monospace");
    view.buffer().set_text(text);

    let scrolled = gtk::ScrolledWindow::new();
    scrolled.set_child(Some(&view));
    scrolled.set_vexpand(true);
    scrolled.set_hexpand(true);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&adw::HeaderBar::new());
    toolbar_view.set_content(Some(&scrolled));

    adw::NavigationPage::builder()
        .title(name)
        .child(&toolbar_view)
        .build()
}

fn show_license_dialog(widget: &impl IsA<gtk::Widget>, licenses: &[(String, String)]) {
    let window = widget.root().and_downcast::<gtk::Window>();

    let nav_view = adw::NavigationView::new();

    if licenses.len() == 1 {
        let (name, text) = &licenses[0];
        nav_view.push(&build_license_text_page(name, text));
    } else {
        let list_box = gtk::ListBox::new();
        list_box.set_selection_mode(gtk::SelectionMode::None);
        list_box.set_valign(gtk::Align::Start);
        list_box.add_css_class("boxed-list");
        list_box.set_margin_top(18);
        list_box.set_margin_bottom(18);
        list_box.set_margin_start(18);
        list_box.set_margin_end(18);

        let licenses_owned: Vec<(String, String)> = licenses.to_vec();

        for (name, _text) in &licenses_owned {
            let row = adw::ActionRow::builder()
                .title(name)
                .activatable(true)
                .build();
            row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
            list_box.append(&row);
        }

        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_child(Some(&list_box));
        scrolled.set_vexpand(true);

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&adw::HeaderBar::new());
        toolbar_view.set_content(Some(&scrolled));

        let list_page = adw::NavigationPage::builder()
            .title("Licenses")
            .child(&toolbar_view)
            .build();

        nav_view.push(&list_page);

        let nav_view_weak = nav_view.downgrade();
        list_box.connect_row_activated(move |_, row| {
            let Some(nav_view) = nav_view_weak.upgrade() else {
                return;
            };
            let index = row.index() as usize;
            if let Some((name, text)) = licenses_owned.get(index) {
                nav_view.push(&build_license_text_page(name, text));
            }
        });
    }

    let dialog = adw::Dialog::builder()
        .child(&nav_view)
        .content_width(700)
        .content_height(500)
        .build();

    dialog.present(window.as_ref());
}

fn open_url(widget: &impl IsA<gtk::Widget>, url: &str) {
    let launcher = gtk::UriLauncher::new(url);
    let window = widget.root().and_downcast::<gtk::Window>();
    launcher.launch(window.as_ref(), gio::Cancellable::NONE, |result| {
        if let Err(err) = result {
            tracing::warn!("Failed to open URL: {err}");
        }
    });
}
