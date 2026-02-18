use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, gio, glib};
use std::cell::{Cell, RefCell};

use crate::app_tile::NscAppTile;

#[derive(Debug, CompositeTemplate)]
#[template(resource = "/org/snowflakeos/NixSoftwareCenter/ui/search_page.ui")]
pub struct SearchPage {
    #[template_child]
    pub results_stack: TemplateChild<gtk::Stack>,
    #[template_child]
    pub status_page: TemplateChild<adw::StatusPage>,
    #[template_child]
    pub grid_view: TemplateChild<gtk::GridView>,
    pub model: gio::ListStore,
    pub pool: RefCell<Option<libappstream::Pool>>,
    pub search_generation: Cell<u64>,
}

impl Default for SearchPage {
    fn default() -> Self {
        Self {
            results_stack: TemplateChild::default(),
            status_page: TemplateChild::default(),
            grid_view: TemplateChild::default(),
            model: gio::ListStore::new::<libappstream::Component>(),
            pool: RefCell::default(),
            search_generation: Cell::new(0),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for SearchPage {
    const NAME: &'static str = "NscSearchPage";
    type Type = super::SearchPage;
    type ParentType = adw::Bin;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for SearchPage {
    fn constructed(&self) {
        self.parent_constructed();

        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("list item must be a ListItem");
            let tile: NscAppTile = glib::Object::new();
            list_item.set_child(Some(&tile));
        });

        factory.connect_bind(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("list item must be a ListItem");
            let component = list_item
                .item()
                .and_downcast::<libappstream::Component>()
                .expect("item must be a Component");
            let tile = list_item
                .child()
                .and_downcast::<NscAppTile>()
                .expect("child must be an NscAppTile");
            tile.bind(&component);
        });

        factory.connect_unbind(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("list item must be a ListItem");
            if let Some(tile) = list_item.child().and_downcast::<NscAppTile>() {
                tile.unbind();
            }
        });

        let selection = gtk::NoSelection::new(Some(self.model.clone()));
        self.grid_view.set_model(Some(&selection));
        self.grid_view.set_factory(Some(&factory));
    }
}

impl WidgetImpl for SearchPage {}
impl BinImpl for SearchPage {}
