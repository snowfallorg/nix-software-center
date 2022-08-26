use adw::prelude::*;
use relm4::*;

use super::updatepage::UpdatePageMsg;

#[derive(Debug)]
pub struct UpdateDialogModel {
    hidden: bool,
    message: String,
    done: bool,
    failed: bool,
}

#[derive(Debug)]
pub enum UpdateDialogMsg {
    Show(String),
    Close,
    Done,
    Failed,
}

#[relm4::component(pub)]
impl SimpleComponent for UpdateDialogModel {
    type InitParams = gtk::Window;
    type Input = UpdateDialogMsg;
    type Output = UpdatePageMsg;
    type Widgets = UpdateDialogWidgets;

    view! {
        dialog = adw::Window {
            #[watch]
            set_visible: !model.hidden,
            set_transient_for: Some(&parent_window),
            set_modal: true,
            set_resizable: false,
            set_default_width: 500,
            set_default_height: 200,
            add_css_class: "dialog",
            add_css_class: "message",
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_halign: gtk::Align::Fill,
                set_valign: gtk::Align::Fill,
                set_hexpand: true,
                set_vexpand: true,
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,
                    set_vexpand: true,
                    set_margin_all: 15,
                    set_spacing: 10,
                    gtk::Label {
                        #[watch]
                        set_visible: !model.message.is_empty(),
                        add_css_class: "title-1",
                        #[watch]
                        set_label: &model.message,
                    },
                    if model.done {
                        gtk::Image {
                            add_css_class: "success",
                            set_icon_name: Some("emblem-ok-symbolic"),
                            set_pixel_size: 128,
                        }
                    } else if model.failed {
                        gtk::Image {
                            add_css_class: "error",
                            set_icon_name: Some("dialog-error-symbolic"),
                            set_pixel_size: 128,
                        }
                    } else {
                        gtk::Spinner {
                            #[watch]
                            set_visible: !model.done,
                            #[watch]
                            set_spinning: !model.done,
                        }
                    }
                },                
                gtk::Box {
                    #[watch]
                    set_visible: model.done || model.failed,
                    add_css_class: "dialog-action-area",
                    set_valign: gtk::Align::End,
                    set_vexpand: true,
                    set_orientation: gtk::Orientation::Horizontal,
                    set_homogeneous: true,
                    gtk::Button {
                        set_label: "Close",
                        connect_clicked[sender] => move |_| {
                            sender.input(UpdateDialogMsg::Close);
                        }
                    }
                },
            }
        }
    }

    fn init(
        parent_window: Self::InitParams,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = UpdateDialogModel {
            hidden: true,
            done: false,
            failed: false,
            message: String::default(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            UpdateDialogMsg::Show(desc) => {
                self.message = desc;
                self.hidden = false;
                self.done = false;
                self.failed = false;
            }
            UpdateDialogMsg::Close => {
                self.hidden = true;
            }
            UpdateDialogMsg::Done => {
                self.done = true;
            }
            UpdateDialogMsg::Failed => {
                self.failed = true;
            }
        }
    }
}