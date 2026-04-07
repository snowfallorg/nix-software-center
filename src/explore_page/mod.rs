mod imp;

use std::collections::HashSet;

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use libappstream::prelude::*;

use crate::app_tile::NscAppTile;

glib::wrapper! {
    pub struct ExplorePage(ObjectSubclass<imp::ExplorePage>)
        @extends adw::BreakpointBin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ExplorePage {
    pub fn refresh_badges(&self) {
        let flow_box = &self.imp().flow_box;
        let mut child = flow_box.first_child();
        while let Some(widget) = child {
            if let Some(flow_child) = widget.downcast_ref::<gtk::FlowBoxChild>()
                && let Some(tile) = flow_child.child().and_downcast::<NscAppTile>()
            {
                tile.refresh_badge();
            }
            child = widget.next_sibling();
        }
    }

    pub fn populate(
        &self,
        all_components: &[libappstream::Component],
        nixos_attrs: &HashSet<String>,
        hm_attrs: &HashSet<String>,
        profile_attrs: &HashSet<String>,
        desktop_ids: &HashSet<String>,
        unavailable: &HashSet<String>,
    ) {
        let flow_box = &self.imp().flow_box;

        while let Some(child) = flow_box.first_child() {
            flow_box.remove(&child);
        }

        let mut components: Vec<_> = all_components
            .iter()
            .filter(|c| {
                !c.icons().is_empty()
                    && !c.screenshots_all().is_empty()
                    && c.kind() == libappstream::ComponentKind::DesktopApp
                    && !c
                        .pkgname()
                        .is_some_and(|p| unavailable.contains(p.as_str()))
            })
            .collect();
        tracing::debug!(
            "Explore: {}/{} components match explore criteria",
            components.len(),
            all_components.len()
        );
        fastrand::shuffle(&mut components);

        for component in components.iter().take(12) {
            let tile =
                NscAppTile::new(component, nixos_attrs, hm_attrs, profile_attrs, desktop_ids);
            flow_box.append(&tile);

            let flow_child = tile
                .parent()
                .expect("tile must have a FlowBoxChild parent")
                .downcast::<gtk::FlowBoxChild>()
                .expect("parent must be a FlowBoxChild");
            flow_child.set_focusable(false);
            flow_child.add_css_class("transparent-container");
        }
    }
}
