use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, gdk, gio, glib};
use libappstream::prelude::ComponentExt;

use crate::app_detail::NscAppDetail;
use crate::application::NscApplication;
use crate::config::{APP_ID, PROFILE};
use crate::explore_page::ExplorePage;
use crate::installed_page::InstalledPage;
use crate::pending_changes::PendingChanges;
use crate::pending_item::{InstallTarget, PendingItem};
use crate::pending_row::NscPendingRow;
use crate::search_page::SearchPage;
use crate::updates_page::UpdatesPage;

#[derive(Debug, CompositeTemplate)]
#[template(resource = "/org/snowflakeos/NixSoftwareCenter/ui/window.ui")]
pub struct NscWindow {
    #[template_child]
    pub split_view: TemplateChild<adw::OverlaySplitView>,
    #[template_child]
    pub headerbar: TemplateChild<adw::HeaderBar>,
    #[template_child]
    pub navigation_view: TemplateChild<adw::NavigationView>,
    #[template_child]
    pub loading_stack: TemplateChild<gtk::Stack>,
    #[template_child]
    pub loading_status: TemplateChild<adw::StatusPage>,
    #[template_child]
    pub view_stack: TemplateChild<adw::ViewStack>,
    #[template_child]
    pub search_button: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    pub refresh_updates_button: TemplateChild<gtk::Button>,
    #[template_child]
    pub sidebar_button: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    pub search_bar: TemplateChild<gtk::SearchBar>,
    #[template_child]
    pub search_entry: TemplateChild<gtk::SearchEntry>,
    #[template_child]
    pub search_page: TemplateChild<SearchPage>,
    #[template_child]
    pub pending_stack: TemplateChild<gtk::Stack>,
    #[template_child]
    pub pending_content_box: TemplateChild<gtk::Box>,
    #[template_child]
    pub sidebar_back_button: TemplateChild<gtk::Button>,
    pub pending_changes: PendingChanges,
    pub settings: gio::Settings,
    pub last_tab: RefCell<String>,
}

impl Default for NscWindow {
    fn default() -> Self {
        Self {
            split_view: TemplateChild::default(),
            headerbar: TemplateChild::default(),
            navigation_view: TemplateChild::default(),
            loading_stack: TemplateChild::default(),
            loading_status: TemplateChild::default(),
            view_stack: TemplateChild::default(),
            refresh_updates_button: TemplateChild::default(),
            search_button: TemplateChild::default(),
            sidebar_button: TemplateChild::default(),
            search_bar: TemplateChild::default(),
            search_entry: TemplateChild::default(),
            search_page: TemplateChild::default(),
            pending_stack: TemplateChild::default(),
            pending_content_box: TemplateChild::default(),
            sidebar_back_button: TemplateChild::default(),
            pending_changes: PendingChanges::default(),
            settings: gio::Settings::new(*APP_ID),
            last_tab: RefCell::new("explore".to_string()),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for NscWindow {
    const NAME: &'static str = "NscWindow";
    type Type = super::NscWindow;
    type ParentType = adw::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        ExplorePage::ensure_type();
        InstalledPage::ensure_type();
        UpdatesPage::ensure_type();
        SearchPage::ensure_type();
        NscPendingRow::ensure_type();

        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for NscWindow {
    fn constructed(&self) {
        self.parent_constructed();
        let obj = self.obj();

        if *PROFILE == "Devel" {
            obj.add_css_class("devel");
        }

        self.loading_status.set_icon_name(Some(*APP_ID));

        // Load latest window state
        obj.load_window_size();

        // Wire up pending changes sidebar
        self.setup_pending_sidebar();

        // Search action activates search; if already active, re-focuses the entry
        let search_button = self.search_button.clone();
        let search_entry = self.search_entry.clone();
        let nav_view = self.navigation_view.clone();
        let action_search = gio::ActionEntry::builder("search")
            .activate(move |win: &super::NscWindow, _, _| {
                if let Some(focus) = gtk::prelude::GtkWindowExt::focus(win)
                    && !gtk::prelude::WidgetExt::is_ancestor(&focus, &nav_view)
                {
                    return;
                }
                search_button.set_active(true);
                search_entry.grab_focus();
            })
            .build();
        obj.add_action_entries([action_search]);

        // Accept keyboard input even when search bar is hidden
        self.search_bar.connect_entry(&*self.search_entry);

        let search_button_ref = self.search_button.clone();
        let search_entry_ref = self.search_entry.clone();
        let search_bar_ref = self.search_bar.clone();
        let nav_view_ref = self.navigation_view.clone();
        let key_controller = gtk::EventControllerKey::new();
        key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        key_controller.connect_key_pressed(move |controller, keyval, _keycode, state| {
            // Only handle keys when focus is within the main content
            if let Some(window) = controller.widget().and_downcast::<gtk::Window>()
                && let Some(focus) = gtk::prelude::GtkWindowExt::focus(&window)
                && !gtk::prelude::WidgetExt::is_ancestor(&focus, &nav_view_ref)
            {
                return glib::Propagation::Proceed;
            }

            // Ignore shortcuts
            let shortcut = state & (gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::ALT_MASK);
            if !shortcut.is_empty() {
                return glib::Propagation::Proceed;
            }

            if let Some(ch) = keyval.to_unicode()
                && !ch.is_control()
            {
                // If search is already active, recapture stray keys to the entry
                if search_bar_ref.is_search_mode() && !search_entry_ref.has_focus() {
                    search_entry_ref.grab_focus();
                    search_entry_ref.delete_selection();
                    let mut pos = search_entry_ref.position();
                    let s = ch.to_string();
                    search_entry_ref.insert_text(&s, &mut pos);
                    search_entry_ref.set_position(pos);
                    return glib::Propagation::Stop;
                }

                // If search is not active, activate it and start typing
                if !search_bar_ref.is_search_mode() {
                    search_button_ref.set_active(true);
                    search_entry_ref.grab_focus();
                    let s = ch.to_string();
                    search_entry_ref.insert_text(&s, &mut 0);
                    search_entry_ref.set_position(s.len() as i32);
                    return glib::Propagation::Stop;
                }
            }
            glib::Propagation::Proceed
        });
        obj.add_controller(key_controller);

        // When search mode is enabled, focus the entry
        // When text is typed, switch to the search page
        // When disabled, restore the previous tab if we're still on the search page
        let view_stack = self.view_stack.clone();
        let search_entry = self.search_entry.clone();
        let last_tab = self.last_tab.clone();
        self.search_bar
            .connect_search_mode_enabled_notify(glib::clone!(
                #[weak]
                view_stack,
                #[weak]
                search_entry,
                move |bar| {
                    if bar.is_search_mode() {
                        if let Some(name) = view_stack.visible_child_name()
                            && name != "search"
                        {
                            *last_tab.borrow_mut() = name.to_string();
                        }
                        search_entry.grab_focus();
                    } else if let Some(name) = view_stack.visible_child_name()
                        && name == "search"
                    {
                        let tab = last_tab.borrow().clone();
                        view_stack.set_visible_child_name(&tab);
                    }
                }
            ));

        // When search text changes, switch to the search page and run the query
        let search_page = self.search_page.clone();
        let search_bar = self.search_bar.clone();
        let view_stack = self.view_stack.clone();
        self.search_entry.connect_search_changed(glib::clone!(
            #[weak]
            search_page,
            #[weak]
            search_bar,
            #[weak]
            view_stack,
            move |entry| {
                if !search_bar.is_search_mode() {
                    return;
                }
                let query = entry.text();
                view_stack.set_visible_child_name("search");
                search_page.perform_search(&query);
            }
        ));

        // Pressing Escape in search entry closes search
        let search_button = self.search_button.clone();
        self.search_entry.connect_stop_search(glib::clone!(
            #[weak]
            search_button,
            move |_| {
                search_button.set_active(false);
            }
        ));

        // Clicking a visible ViewStack tab dismisses search.
        let search_button = self.search_button.clone();
        let refresh_button = self.refresh_updates_button.clone();
        self.view_stack.connect_visible_child_notify(glib::clone!(
            #[weak]
            search_button,
            #[weak]
            refresh_button,
            move |stack| {
                if let Some(name) = stack.visible_child_name() {
                    if name != "search" {
                        search_button.set_active(false);
                    }
                    refresh_button.set_visible(name == "updates");
                }
            }
        ));

        self.refresh_updates_button.connect_clicked(|_| {
            let Some(app) = gio::Application::default().and_downcast::<NscApplication>() else {
                return;
            };
            app.refresh_updates();
        });
    }
}

impl WidgetImpl for NscWindow {}

impl WindowImpl for NscWindow {
    fn close_request(&self) -> glib::Propagation {
        if let Err(err) = self.obj().save_window_size() {
            tracing::warn!("Failed to save window state, {}", &err);
        }

        self.parent_close_request()
    }
}

impl ApplicationWindowImpl for NscWindow {}

impl AdwApplicationWindowImpl for NscWindow {}

impl NscWindow {
    fn setup_pending_sidebar(&self) {
        let pending_changes = &self.pending_changes;
        let pending_stack = self.pending_stack.clone();
        let pending_content_box = self.pending_content_box.clone();
        let sidebar_button = self.sidebar_button.clone();

        let pc = pending_changes.clone();
        let stack = pending_stack.clone();
        let content_box = pending_content_box.clone();
        let btn = sidebar_button.clone();
        let prev_count = std::cell::Cell::new(0u32);
        pending_changes.connect_items_changed(move |_, _, _, _| {
            Self::rebuild_pending_list(&pc, &stack, &content_box);

            let n = pc.n_items();
            let was = prev_count.replace(n);

            if n > 0 {
                btn.set_tooltip_text(Some(&format!("Pending Changes ({})", n)));
                btn.add_css_class("suggested-action");
            } else {
                btn.set_tooltip_text(Some("Pending Changes Sidebar"));
                btn.remove_css_class("suggested-action");
            }

            if n > was {
                Self::shake_widget(&btn);
            }
        });

        let split_view = self.split_view.clone();
        self.sidebar_back_button.connect_clicked(move |_| {
            split_view.set_show_sidebar(false);
        });

        Self::rebuild_pending_list(pending_changes, &pending_stack, &pending_content_box);
    }

    fn rebuild_pending_list(
        pending_changes: &PendingChanges,
        stack: &gtk::Stack,
        content_box: &gtk::Box,
    ) {
        while let Some(child) = content_box.first_child() {
            content_box.remove(&child);
        }

        let n = pending_changes.n_items();

        if n == 0 {
            stack.set_visible_child_name("empty");
            return;
        }

        stack.set_visible_child_name("list");

        let nixos_items = pending_changes.items_for_target(InstallTarget::NixOS);
        let hm_items = pending_changes.items_for_target(InstallTarget::HomeManager);

        if !nixos_items.is_empty() {
            Self::append_section(content_box, "NixOS", &nixos_items, pending_changes);
        }

        if !hm_items.is_empty() {
            Self::append_section(content_box, "Home Manager", &hm_items, pending_changes);
        }
    }

    fn append_section(
        content_box: &gtk::Box,
        title: &str,
        items: &[PendingItem],
        pending_changes: &PendingChanges,
    ) {
        let section = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .build();

        let label = gtk::Label::builder()
            .label(title)
            .xalign(0.0)
            .css_classes(["title-4"])
            .build();
        section.append(&label);

        let list_box = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();

        list_box.connect_row_activated(|_list_box, row| {
            let Some(row) = row.downcast_ref::<NscPendingRow>() else {
                return;
            };
            let Some(component) = row.component() else {
                return;
            };
            let target = row.target();

            let Some(app) = gio::Application::default().and_downcast::<NscApplication>() else {
                return;
            };

            let metadata_ref = app.metadata().borrow();
            let Some(metadata) = metadata_ref.as_ref() else {
                return;
            };

            let Some(window) = row.root().and_downcast::<super::NscWindow>() else {
                return;
            };

            let imp = window.imp();

            let existing_detail = imp
                .navigation_view
                .visible_page()
                .and_downcast::<NscAppDetail>()
                .filter(|detail| {
                    detail
                        .imp()
                        .component
                        .borrow()
                        .as_ref()
                        .and_then(|c| c.pkgname())
                        == component.pkgname()
                });

            let target_index = match target {
                InstallTarget::NixOS => 0,
                InstallTarget::HomeManager => 1,
                InstallTarget::Profile => 2,
            };

            if let Some(detail) = existing_detail {
                detail.imp().target_dropdown.set_selected(target_index);
            } else {
                let nixos_attrs = app.installed_nixos_attrs().borrow();
                let hm_attrs = app.installed_hm_attrs().borrow();
                let profile_attrs = app.installed_profile_attrs().borrow();
                let detail = NscAppDetail::new(
                    &component,
                    metadata,
                    &nixos_attrs,
                    &hm_attrs,
                    &profile_attrs,
                );
                detail.imp().target_dropdown.set_selected(target_index);
                imp.navigation_view.push(&detail);
            }
            imp.split_view.set_show_sidebar(false);
        });

        for item in items {
            let row = NscPendingRow::new(item);
            let pc = pending_changes.clone();
            let target = item.target();
            let item_clone = item.clone();
            row.connect_remove(move |_| {
                if let Some(component) = item_clone.component() {
                    pc.remove_by_component(&component, target);
                }
            });
            list_box.append(&row);
        }

        section.append(&list_box);
        content_box.append(&section);
    }

    pub fn shake_widget(widget: &impl IsA<gtk::Widget>) {
        let w = widget.as_ref().clone();
        let amplitude = 15.0f64;

        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let css_class = format!("nsc-shake-{id}");

        w.add_css_class(&css_class);

        let provider = gtk::CssProvider::new();
        let display = w.display();
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
        );

        let css_class_anim = css_class.clone();
        let provider_anim = provider.clone();
        let target = adw::CallbackAnimationTarget::new(move |value| {
            let oscillation = (value * std::f64::consts::PI * 6.0).sin();
            let decay = 1.0 - value;
            let angle = amplitude * oscillation * decay;
            let css = format!(
                ".{css_class_anim} {{ transition: none; transform: rotate({angle:.2}deg); }}"
            );
            #[allow(deprecated)]
            provider_anim.load_from_data(&css);
        });

        let anim = adw::TimedAnimation::new(&w, 0.0, 1.0, 400, target);
        anim.set_easing(adw::Easing::Linear);

        let w_done = w.clone();
        let display_done = display.clone();
        let provider_done = provider.clone();
        anim.connect_done(move |_| {
            w_done.remove_css_class(&css_class);
            gtk::style_context_remove_provider_for_display(&display_done, &provider_done);
        });

        anim.play();
    }
}
