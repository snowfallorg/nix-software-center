mod imp;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use libappstream::prelude::*;
use std::collections::HashSet;
use std::os::unix::process::CommandExt;
use std::sync::{Arc, Mutex};

use crate::pending_item::InstallTarget;
use crate::window::NscWindow;

#[derive(Debug)]
pub struct RunCancel {
    abort_handle: tokio::task::AbortHandle,
    child_pid: Arc<Mutex<Option<u32>>>,
}

impl RunCancel {
    fn cancel(&self) {
        if let Some(pid) = self.child_pid.lock().unwrap().take() {
            let _ = std::process::Command::new("kill")
                .args(["--", &format!("-{pid}")])
                .status();
        }
        self.abort_handle.abort();
    }
}

glib::wrapper! {
    pub struct NscAppDetail(ObjectSubclass<imp::NscAppDetail>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl NscAppDetail {
    pub fn new(
        component: &libappstream::Component,
        metadata: &libsnow::metadata::Metadata,
        installed_nixos_attrs: &HashSet<String>,
        installed_hm_attrs: &HashSet<String>,
    ) -> Self {
        let page: Self = glib::Object::new();
        page.populate(
            component,
            metadata,
            installed_nixos_attrs,
            installed_hm_attrs,
        );
        page
    }

    fn populate(
        &self,
        component: &libappstream::Component,
        metadata: &libsnow::metadata::Metadata,
        installed_nixos_attrs: &HashSet<String>,
        installed_hm_attrs: &HashSet<String>,
    ) {
        let imp = self.imp();

        imp.component.replace(Some(component.clone()));

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

        // flat needs to be applied to the child rather than the GtkDropDown itself
        if let Some(child) = imp.target_dropdown.first_child() {
            child.add_css_class("flat");
        }

        if let Some(pkgname) = component.pkgname() {
            let pkgname_str = pkgname.as_str();
            let installed_nixos = installed_nixos_attrs.contains(pkgname_str);
            imp.installed_nixos.set(installed_nixos);

            let installed_hm = installed_hm_attrs.contains(pkgname_str);
            imp.installed_hm.set(installed_hm);

            if installed_hm && !installed_nixos {
                imp.target_dropdown.set_selected(1);
            } else {
                imp.target_dropdown.set_selected(0);
            }
        }

        Self::setup_action_buttons(self, imp, component);

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

    fn selected_target(imp: &imp::NscAppDetail) -> InstallTarget {
        match imp.target_dropdown.selected() {
            0 => InstallTarget::NixOS,
            _ => InstallTarget::HomeManager,
        }
    }

    fn is_installed_for_target(imp: &imp::NscAppDetail, target: InstallTarget) -> bool {
        match target {
            InstallTarget::NixOS => imp.installed_nixos.get(),
            InstallTarget::HomeManager => imp.installed_hm.get(),
        }
    }

    fn setup_action_buttons(
        page: &Self,
        imp: &imp::NscAppDetail,
        component: &libappstream::Component,
    ) {
        let component_install = component.clone();
        let page_weak_install = page.downgrade();
        imp.install_button.connect_clicked(move |_button| {
            let Some(page) = page_weak_install.upgrade() else {
                return;
            };
            let imp = page.imp();
            let target = Self::selected_target(imp);
            let installed = Self::is_installed_for_target(imp, target);

            if installed {
                Self::launch_app(&component_install);
                return;
            }

            let Some(window) = page
                .root()
                .and_downcast::<adw::ApplicationWindow>()
                .and_then(|w| w.downcast::<NscWindow>().ok())
            else {
                return;
            };

            let pending = window.pending_changes();
            if pending.contains(&component_install, target) {
                pending.remove_by_component(&component_install, target);
            } else {
                pending.add_install(&component_install, target);
            }
        });

        let component_remove = component.clone();
        let page_weak_remove = page.downgrade();
        imp.trash_button.connect_clicked(move |_button| {
            let Some(page) = page_weak_remove.upgrade() else {
                return;
            };
            let Some(window) = page
                .root()
                .and_downcast::<adw::ApplicationWindow>()
                .and_then(|w| w.downcast::<NscWindow>().ok())
            else {
                return;
            };

            let target = Self::selected_target(page.imp());
            let pending = window.pending_changes();
            if pending.contains(&component_remove, target) {
                pending.remove_by_component(&component_remove, target);
            } else {
                pending.add_remove(&component_remove, target);
            }
        });

        let component_run = component.clone();
        let page_weak_run = page.downgrade();
        imp.run_button.connect_clicked(move |button| {
            let Some(page) = page_weak_run.upgrade() else {
                return;
            };
            let imp = page.imp();

            // If a build is running, cancel it
            if let Some(cancel) = imp.run_cancel.borrow_mut().take() {
                cancel.cancel();
                Self::reset_run_button(button);
                return;
            }

            let Some(pkgname) = component_run.pkgname() else {
                return;
            };

            button.set_tooltip_text(Some("Cancel"));
            button.set_icon_name("");
            let spinner = adw::Spinner::builder()
                .margin_start(10)
                .margin_end(10)
                .margin_top(10)
                .margin_bottom(10)
                .build();
            button.set_child(Some(&spinner));

            let attr = pkgname.to_string();
            let component_for_launch = component_run.clone();
            let (sender, receiver) = async_channel::bounded(1);

            let child_pid: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
            let child_pid_task = child_pid.clone();

            let join_handle = crate::runtime::runtime().spawn(async move {
                let output = match tokio::process::Command::new("nix")
                    .args([
                        "build",
                        &format!("nixpkgs#{attr}"),
                        "--no-link",
                        "--print-out-paths",
                    ])
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .process_group(0)
                    .spawn()
                {
                    Ok(child) => {
                        if let Some(pid) = child.id() {
                            *child_pid_task.lock().unwrap() = Some(pid);
                        }
                        child.wait_with_output().await
                    }
                    Err(e) => Err(e),
                };
                let _ = sender.send(output).await;
            });

            imp.run_cancel.replace(Some(RunCancel {
                abort_handle: join_handle.abort_handle(),
                child_pid,
            }));

            let btn = button.clone();
            let page_weak = page.downgrade();
            glib::spawn_future_local(async move {
                if let Ok(result) = receiver.recv().await {
                    if let Some(page) = page_weak.upgrade() {
                        page.imp().run_cancel.replace(None);
                    }

                    Self::reset_run_button(&btn);

                    match result {
                        Ok(output) if output.status.success() => {
                            let store_path =
                                String::from_utf8_lossy(&output.stdout).trim().to_string();
                            Self::launch_from_store_path(&store_path, &component_for_launch);
                        }
                        Ok(output) => {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            tracing::warn!("nix build failed: {stderr}");
                        }
                        Err(err) => {
                            tracing::warn!("Failed to run nix build: {err}");
                        }
                    }
                }
            });
        });

        let page_weak_dd = page.downgrade();
        imp.target_dropdown
            .connect_selected_notify(move |_dropdown| {
                if let Some(page) = page_weak_dd.upgrade() {
                    Self::sync_button_states(&page);
                }
            });

        let page_weak_unmap = page.downgrade();
        page.connect_unmap(move |_| {
            if let Some(page) = page_weak_unmap.upgrade() {
                Self::cancel_run_build(page.imp());
                Self::disconnect_pending_changed(page.imp());
            }
        });

        let page_weak_map = page.downgrade();
        page.connect_map(move |_| {
            let Some(page) = page_weak_map.upgrade() else {
                return;
            };
            let Some(window) = page
                .root()
                .and_downcast::<adw::ApplicationWindow>()
                .and_then(|w| w.downcast::<NscWindow>().ok())
            else {
                return;
            };

            Self::sync_button_states(&page);
            Self::sync_sidebar_button_style(&page.imp().sidebar_button, window.pending_changes());
            Self::ensure_pending_changed_connected(&page, &window);
        });
    }

    fn ensure_pending_changed_connected(page: &Self, window: &NscWindow) {
        let imp = page.imp();
        if imp.pending_changed_handler.borrow().is_some() {
            return;
        }

        let pending = window.pending_changes();
        let page_weak = page.downgrade();
        let prev_count = std::cell::Cell::new(pending.n_items());
        let handler_id = pending.connect_items_changed(move |pc, _, _, _| {
            let Some(page) = page_weak.upgrade() else {
                return;
            };
            Self::sync_button_states(&page);

            let n = pc.n_items();
            let was = prev_count.replace(n);
            Self::sync_sidebar_button_style(&page.imp().sidebar_button, pc);
            if n > was {
                crate::window::NscWindow::shake_widget(&*page.imp().sidebar_button);
            }
        });
        imp.pending_changed_handler
            .replace(Some((handler_id, pending.clone())));
    }

    fn disconnect_pending_changed(imp: &imp::NscAppDetail) {
        if let Some((handler_id, pending)) = imp.pending_changed_handler.take() {
            pending.disconnect(handler_id);
        }
    }

    fn sync_button_states(page: &Self) {
        let Some(window) = page
            .root()
            .and_downcast::<adw::ApplicationWindow>()
            .and_then(|w| w.downcast::<NscWindow>().ok())
        else {
            return;
        };

        let imp = page.imp();
        let Some(component) = imp.component.borrow().clone() else {
            return;
        };

        let target = Self::selected_target(imp);
        let installed = Self::is_installed_for_target(imp, target);
        let pending = window.pending_changes();
        let is_pending = pending.contains(&component, target);

        if installed {
            imp.install_button.set_visible(true);
            imp.install_button.set_label("Open");
            imp.install_button.remove_css_class("destructive-action");
            imp.install_button.add_css_class("suggested-action");

            imp.trash_button.set_visible(true);
            imp.run_button.set_visible(false);

            if is_pending {
                imp.trash_button.add_css_class("destructive-action");
                imp.trash_button.set_tooltip_text(Some("Undo Removal"));
            } else {
                imp.trash_button.remove_css_class("destructive-action");
                imp.trash_button.set_tooltip_text(Some("Remove"));
            }
        } else {
            imp.trash_button.set_visible(false);
            imp.run_button.set_visible(true);

            imp.install_button.set_visible(true);
            if is_pending {
                imp.install_button.set_label("Pending");
                imp.install_button.remove_css_class("suggested-action");
                imp.install_button.add_css_class("destructive-action");
            } else {
                imp.install_button.set_label("Install");
                imp.install_button.remove_css_class("destructive-action");
                imp.install_button.add_css_class("suggested-action");
            }
        }
    }

    fn sync_sidebar_button_style(
        button: &gtk::ToggleButton,
        pending: &crate::pending_changes::PendingChanges,
    ) {
        if pending.n_items() > 0 {
            button.add_css_class("suggested-action");
        } else {
            button.remove_css_class("suggested-action");
        }
    }

    fn reset_run_button(button: &gtk::Button) {
        button.set_child(None::<&gtk::Widget>);
        button.set_icon_name("media-playback-start-symbolic");
        button.set_sensitive(true);
        button.set_tooltip_text(Some("Run without installing"));
    }

    fn cancel_run_build(imp: &imp::NscAppDetail) {
        if let Some(cancel) = imp.run_cancel.borrow_mut().take() {
            cancel.cancel();
            Self::reset_run_button(&imp.run_button);
        }
    }

    fn launch_app(component: &libappstream::Component) {
        let desktop_id = component
            .launchable(libappstream::LaunchableKind::DesktopId)
            .and_then(|l| l.entries().into_iter().next())
            .or_else(|| component.id());

        let Some(id) = desktop_id else {
            tracing::warn!("No desktop ID found for component");
            return;
        };

        let Some(app_info) = gio::DesktopAppInfo::new(&id) else {
            tracing::warn!("Could not find desktop file for {id}");
            return;
        };

        if let Err(err) = app_info.launch(&[], gio::AppLaunchContext::NONE) {
            tracing::warn!("Failed to launch {id}: {err}");
        }
    }

    fn launch_from_store_path(store_path: &str, component: &libappstream::Component) {
        use gio::prelude::*;

        let apps_dir = std::path::Path::new(store_path).join("share/applications");
        let Ok(entries) = std::fs::read_dir(&apps_dir) else {
            tracing::warn!("No share/applications in {store_path}");
            return;
        };

        let desktop_files: Vec<std::path::PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "desktop"))
            .collect();

        if desktop_files.is_empty() {
            tracing::warn!("No .desktop files found in {}", apps_dir.display());
            return;
        }

        let preferred_id = component
            .launchable(libappstream::LaunchableKind::DesktopId)
            .and_then(|l| l.entries().into_iter().next())
            .or_else(|| component.id());

        let desktop_file = if let Some(ref id) = preferred_id {
            desktop_files
                .iter()
                .find(|p| {
                    p.file_name()
                        .is_some_and(|name| name.to_string_lossy() == id.as_str())
                })
                .unwrap_or(&desktop_files[0])
        } else {
            &desktop_files[0]
        };

        let keyfile = glib::KeyFile::new();
        if let Err(err) = keyfile.load_from_file(desktop_file, glib::KeyFileFlags::NONE) {
            tracing::warn!("Failed to load {}: {err}", desktop_file.display());
            return;
        }

        let bin_dir = std::path::Path::new(store_path).join("bin");
        let bin_dir_str = bin_dir.to_string_lossy().to_string();
        let share_dir = format!("{store_path}/share");
        let new_path = format!(
            "{bin_dir_str}:{}",
            std::env::var("PATH").unwrap_or_default()
        );
        let new_xdg = format!(
            "{share_dir}:{}",
            std::env::var("XDG_DATA_DIRS").unwrap_or_default()
        );

        let resolve_store_command = |command: &str| {
            if command.contains('/') {
                return Some(command.to_string());
            }
            let candidate = bin_dir.join(command);
            if candidate.exists() {
                Some(candidate.to_string_lossy().to_string())
            } else {
                None
            }
        };

        if let Ok(try_exec) = keyfile.string("Desktop Entry", "TryExec")
            && let Some(resolved_try_exec) = resolve_store_command(try_exec.as_str())
        {
            keyfile.set_string("Desktop Entry", "TryExec", &resolved_try_exec);
        }

        if let Ok(exec_line) = keyfile.string("Desktop Entry", "Exec") {
            let trimmed = exec_line.trim_start();
            if let Some(command) = trimmed.split_whitespace().next()
                && let Some(resolved) = resolve_store_command(command)
                && let Some(command_offset) = exec_line.find(command)
            {
                let command_end = command_offset + command.len();
                let rewritten_exec = format!(
                    "{}{}{}",
                    &exec_line[..command_offset],
                    resolved,
                    &exec_line[command_end..]
                );
                keyfile.set_string("Desktop Entry", "Exec", &rewritten_exec);
            }
        }

        let app_info = gio::DesktopAppInfo::from_keyfile(&keyfile);

        if let Some(app_info) = app_info {
            let ctx = gio::AppLaunchContext::new();
            ctx.setenv("PATH", &new_path);
            ctx.setenv("XDG_DATA_DIRS", &new_xdg);

            match app_info.launch(&[], Some(&ctx)) {
                Ok(()) => return,
                Err(err) => {
                    tracing::warn!("gio launch failed, falling back to Exec: {err}");
                }
            }
        }

        tracing::info!(
            "Falling back to parsing Exec= for {}",
            desktop_file.display()
        );
        let Ok(exec_line) = keyfile.string("Desktop Entry", "Exec") else {
            tracing::warn!("No Exec= in {}", desktop_file.display());
            return;
        };

        let argv: Vec<&str> = exec_line
            .split_whitespace()
            .filter(|arg| !arg.starts_with('%'))
            .collect();

        let Some(command) = argv.first() else {
            tracing::warn!("Empty Exec= in {}", desktop_file.display());
            return;
        };

        let resolved = if !command.contains('/') {
            let candidate = bin_dir.join(command);
            if candidate.exists() {
                candidate.to_string_lossy().to_string()
            } else {
                command.to_string()
            }
        } else {
            command.to_string()
        };

        let mut cmd = std::process::Command::new(&resolved);
        cmd.args(&argv[1..])
            .env("PATH", &new_path)
            .env("XDG_DATA_DIRS", &new_xdg)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .process_group(0);

        if let Err(err) = cmd.spawn() {
            tracing::warn!("Failed to launch {resolved}: {err}");
        }
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
