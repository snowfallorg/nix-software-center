mod imp;

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use libappstream::prelude::*;
use rand::seq::SliceRandom;

use crate::app_tile::NscAppTile;

glib::wrapper! {
    pub struct ExplorePage(ObjectSubclass<imp::ExplorePage>)
        @extends adw::BreakpointBin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ExplorePage {
    pub fn populate(&self, _metadata: &libsnow::metadata::Metadata, pool: &libappstream::Pool) {
        let flow_box = &self.imp().flow_box;

        while let Some(child) = flow_box.first_child() {
            flow_box.remove(&child);
        }

        let Some(cbox) = pool.components() else {
            return;
        };

        let array = cbox.as_array();
        let mut components: Vec<_> = array
            .iter()
            .filter(|c| !c.icons().is_empty() && !c.screenshots_all().is_empty())
            .collect();
        tracing::debug!(
            "Explore: {}/{} components have icons and screenshots",
            components.len(),
            array.len()
        );
        components.shuffle(&mut rand::rng());

        for component in components.iter().take(12) {
            let tile = NscAppTile::new(component);
            flow_box.append(&tile);

            let flow_child = tile
                .parent()
                .expect("tile must have a FlowBoxChild parent")
                .downcast::<gtk::FlowBoxChild>()
                .expect("parent must be a FlowBoxChild");
            flow_child.add_css_class("transparent-container");
        }
    }
}
