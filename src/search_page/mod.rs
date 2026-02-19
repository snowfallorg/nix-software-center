mod imp;

use adw::subclass::prelude::*;
use gtk::glib;
use libappstream::prelude::*;

glib::wrapper! {
    pub struct SearchPage(ObjectSubclass<imp::SearchPage>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

const POPULATE_BATCH_SIZE: usize = 50;

impl SearchPage {
    pub fn set_pool(&self, pool: &libappstream::Pool) {
        self.imp().pool.replace(Some(pool.clone()));
    }

    pub fn perform_search(&self, query: &str) {
        let imp = self.imp();

        if query.is_empty() {
            // Keep showing whatever results are already on screen
            return;
        }

        let generation = imp.search_generation.get().wrapping_add(1);
        imp.search_generation.set(generation);

        imp.model.remove_all();

        let pool = imp.pool.borrow();
        let Some(pool) = pool.as_ref() else {
            imp.results_stack.set_visible_child_name("loading");
            return;
        };

        let results = pool.search(query);

        let Some(cbox) = &results else {
            imp.results_stack.set_visible_child_name("no-results");
            return;
        };

        let array = cbox.as_array();
        let components: Vec<libappstream::Component> = array
            .iter()
            .filter(|c| {
                c.kind() == libappstream::ComponentKind::DesktopApp
                    || c.kind() == libappstream::ComponentKind::ConsoleApp
            })
            .cloned()
            .collect();

        if components.is_empty() {
            imp.results_stack.set_visible_child_name("no-results");
            return;
        }

        imp.results_stack.set_visible_child_name("results");

        let page = self.clone();
        glib::spawn_future_local(async move {
            let imp = page.imp();

            if imp.search_generation.get() != generation {
                return;
            }

            for chunk in components.chunks(POPULATE_BATCH_SIZE) {
                if imp.search_generation.get() != generation {
                    return;
                }
                for component in chunk {
                    imp.model.append(component);
                }
                glib::timeout_future(std::time::Duration::ZERO).await;
            }
        });
    }
}
