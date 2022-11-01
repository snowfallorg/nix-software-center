use super::window::AppMsg;
use adw::prelude::*;
use log::{info, trace};
use relm4::*;
use sourceview5::prelude::*;

#[tracker::track]
pub struct RebuildModel {
    hidden: bool,
    text: String,
    status: RebuildStatus,
    config: String,
    path: String,
    flake: Option<String>,
    scheme: Option<sourceview5::StyleScheme>,
}

#[derive(Debug)]
pub enum RebuildMsg {
    Show,
    FinishSuccess,
    FinishError(Option<String>),
    UpdateText(String),
    Close,
    SetScheme(String),
    Quit,
}

#[derive(PartialEq)]
enum RebuildStatus {
    Building,
    Success,
    Error,
}

#[relm4::component(pub)]
impl SimpleComponent for RebuildModel {
    type Init = gtk::Window;
    type Input = RebuildMsg;
    type Output = AppMsg;
    type Widgets = RebuildWidgets;

    view! {
        dialog = adw::Window {
            set_transient_for: Some(&parent_window),
            set_modal: true,
            #[track(model.changed(RebuildModel::hidden()))]
            set_default_width: 500,
            #[track(model.changed(RebuildModel::hidden()))]
            set_default_height: 200,//295),
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            add_css_class: "dialog",
            add_css_class: "message",
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                #[name(statusstack)]
                gtk::Stack {
                    set_margin_top: 20,
                    set_transition_type: gtk::StackTransitionType::Crossfade,
                    set_vhomogeneous: false,
                    #[name(building)]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 10,
                        gtk::Spinner {
                            #[watch]
                            set_spinning: true,
                            set_height_request: 60,
                        },
                        gtk::Label {
                            set_label: "Building...",
                            add_css_class: "title-1",
                        },
                    },
                    #[name(success)]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 10,
                        gtk::Image {
                            add_css_class: "success",
                            set_icon_name: Some("object-select-symbolic"),
                            set_pixel_size: 128,
                        },
                        gtk::Label {
                            set_label: "Done!",
                            add_css_class: "title-1",
                        },
                        gtk::Label {
                            set_label: "Rebuild successful!",
                            add_css_class: "dim-label",
                        }
                    },
                    #[name(error)]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 10,
                        gtk::Image {
                            add_css_class: "error",
                            set_icon_name: Some("dialog-error-symbolic"),
                            set_pixel_size: 128,
                        },
                        gtk::Label {
                            set_label: "Error!",
                            add_css_class: "title-1",
                        },
                        gtk::Label {
                            set_label: "Rebuild failed! See below for error message.",
                            add_css_class: "dim-label",
                        }
                    }
                },
                gtk::Frame {
                    set_margin_all: 20,
                    #[name(scrollwindow)]
                    gtk::ScrolledWindow {
                        set_max_content_height: 500,
                        set_min_content_height: 100,
                        #[name(outview)]
                        sourceview5::View {
                            set_editable: false,
                            set_cursor_visible: false,
                            set_monospace: true,
                            set_top_margin: 5,
                            set_bottom_margin: 5,
                            set_left_margin: 5,
                            set_vexpand: true,
                            set_hexpand: true,
                            set_vscroll_policy: gtk::ScrollablePolicy::Minimum,
                            #[wrap(Some)]
                            set_buffer: outbuf = &sourceview5::Buffer {
                                #[track(model.changed(RebuildModel::scheme()))]
                                set_style_scheme: model.scheme.as_ref(),
                                #[track(model.changed(RebuildModel::text()))]
                                set_text: &model.text,
                            }
                        }
                    }
                },
                gtk::Box {
                    add_css_class: "dialog-action-area",
                    set_orientation: gtk::Orientation::Horizontal,
                    set_homogeneous: true,
                    #[track(model.changed(RebuildModel::status()))]
                    set_visible: model.status != RebuildStatus::Building,
                    gtk::Button {
                        set_label: "Close",
                        #[track(model.changed(RebuildModel::status()))]
                        set_visible: model.status != RebuildStatus::Building,
                        connect_clicked[sender] => move |_| {
                            sender.input(RebuildMsg::Close)
                        }
                    }
                }
            }
        }
    }

    fn pre_view() {
        match model.status {
            RebuildStatus::Building => {
                statusstack.set_visible_child(building);
            }
            RebuildStatus::Success => statusstack.set_visible_child(success),
            RebuildStatus::Error => statusstack.set_visible_child(error),
        }
    }

    fn post_view() {
        let adj = scrollwindow.vadjustment();
        if model.status == RebuildStatus::Building {
            adj.set_upper(adj.upper() + 20.0);
        }
        adj.set_value(adj.upper());
        if model.status != RebuildStatus::Building {
            outview.scroll_to_mark(&outview.buffer().get_insert(), 0.0, true, 0.0, 0.0);
            scrollwindow.hadjustment().set_value(0.0);
        }
    }

    fn init(
        parent_window: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {

        let model = RebuildModel {
            hidden: true,
            text: String::new(),
            status: RebuildStatus::Building,
            config: String::new(),
            path: String::new(),
            flake: None,
            scheme: None,
            tracker: 0,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.reset();
        match msg {
            RebuildMsg::Show => {
                self.update_hidden(|x| *x = false);
                self.update_text(|x| x.clear());
                self.set_status(RebuildStatus::Building);
            }
            RebuildMsg::UpdateText(s) => {
                info!("RebuildMsg::UpdateText({})", s);
                let newtext = if self.text.is_empty() {
                    s
                } else {
                    format!("{}\n{}", self.text, s)
                };
                self.set_text(newtext);
                trace!("NEWTEXT: {}", self.text);
            }
            RebuildMsg::FinishSuccess => {
                self.set_status(RebuildStatus::Success);
            }
            RebuildMsg::FinishError(msg) => {
                if let Some(s) = msg {
                    self.set_text(s)
                }
                self.update_hidden(|x| *x = false);
                self.set_status(RebuildStatus::Error);
            }
            RebuildMsg::Close => {
                self.update_hidden(|x| *x = true);
                self.update_text(|x| x.clear());
            }
            RebuildMsg::SetScheme(scheme) => {
                self.set_scheme(sourceview5::StyleSchemeManager::default().scheme(&scheme));
            }
            RebuildMsg::Quit => {
                sender.output(AppMsg::Close);
            }
        }
    }
}
