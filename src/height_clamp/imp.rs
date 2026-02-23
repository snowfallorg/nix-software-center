use std::cell::{Cell, RefCell};

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk, glib, graphene, gsk};

const FADE_HEIGHT: f32 = 75.0;

/// If the child is only slightly taller than max-height don't bother clamping, just show it
const CLAMP_LEEWAY: i32 = 30;

#[derive(Debug, glib::Properties)]
#[properties(wrapper_type = super::NscHeightClamp)]
pub struct NscHeightClamp {
    #[property(get, set = Self::set_max_height, explicit_notify, default = 300, minimum = -1)]
    max_height: Cell<i32>,

    #[property(get)]
    will_change: Cell<bool>,

    #[property(get, set = Self::set_child, explicit_notify, nullable)]
    child: RefCell<Option<gtk::Widget>>,

    expanding: Cell<bool>,
    current_height: Cell<f32>,
    animation: RefCell<Option<adw::TimedAnimation>>,
}

impl Default for NscHeightClamp {
    fn default() -> Self {
        Self {
            max_height: Cell::new(300),
            will_change: Cell::new(false),
            child: RefCell::new(None),
            expanding: Cell::new(false),
            current_height: Cell::new(0.0),
            animation: RefCell::new(None),
        }
    }
}

impl NscHeightClamp {
    fn set_child(&self, widget: Option<&gtk::Widget>) {
        if widget == self.child.borrow().as_ref() {
            return;
        }
        if let Some(old) = self.child.borrow_mut().take() {
            old.unparent();
        }
        if let Some(w) = widget {
            self.child.replace(Some(w.clone()));
            w.set_parent(&*self.obj());
        }
        self.obj().queue_resize();
        self.obj().notify_child();
    }

    fn set_max_height(&self, value: i32) {
        if self.max_height.get() == value {
            return;
        }
        self.max_height.set(value);
        self.obj().queue_resize();
        self.obj().notify_max_height();
    }

    /// Update `will-change`. Skips when expanded (`max-height == -1`) to keep the toggle visible
    fn update_will_change(&self, child_natural: i32) {
        let max_h = self.max_height.get();
        if max_h < 0 {
            return;
        }
        let new_val = child_natural > max_h + CLAMP_LEEWAY;
        if self.will_change.get() != new_val {
            self.will_change.set(new_val);
            self.obj().notify_will_change();
        }
    }

    fn is_animating(&self) -> bool {
        self.animation.borrow().is_some()
    }

    /// Compute the target height for this widget given the child's natural height.
    fn target_height(&self, child_natural: i32) -> i32 {
        if self.is_animating() {
            return self.current_height.get() as i32;
        }

        let max_h = self.max_height.get();
        if max_h < 0 || child_natural <= max_h + CLAMP_LEEWAY {
            child_natural
        } else {
            max_h
        }
    }

    /// Animate max-height from current visible height to `new_max_height`.
    pub(super) fn animate_max_height(&self, new_max_height: i32) {
        if let Some(anim) = self.animation.borrow_mut().take() {
            anim.skip();
        }

        let width = self.obj().width();
        let child_natural = self
            .child
            .borrow()
            .as_ref()
            .map(|c| {
                let (_, nat, _, _) = c.measure(gtk::Orientation::Vertical, width);
                nat
            })
            .unwrap_or(0);

        let from_h = self.target_height(child_natural);

        self.max_height.set(new_max_height);
        self.obj().notify_max_height();

        let to_h = self.target_height(child_natural);

        if from_h == to_h {
            self.obj().queue_resize();
            return;
        }

        let is_expanding = to_h > from_h;

        let obj_weak = self.obj().downgrade();
        let target = adw::CallbackAnimationTarget::new(move |value| {
            let Some(obj) = obj_weak.upgrade() else {
                return;
            };
            obj.imp().current_height.set(value as f32);
            obj.queue_resize();
        });

        let anim = adw::TimedAnimation::new(&*self.obj(), from_h as f64, to_h as f64, 250, target);
        anim.set_easing(adw::Easing::EaseOutCubic);

        let obj_weak = self.obj().downgrade();
        anim.connect_done(move |_| {
            let Some(obj) = obj_weak.upgrade() else {
                return;
            };
            obj.imp().animation.borrow_mut().take();
            obj.queue_resize();
        });

        self.expanding.set(is_expanding);
        self.current_height.set(from_h as f32);
        anim.play();
        self.animation.borrow_mut().replace(anim);
    }
}

#[glib::object_subclass]
impl ObjectSubclass for NscHeightClamp {
    const NAME: &'static str = "NscHeightClamp";
    type ParentType = gtk::Widget;
    type Type = super::NscHeightClamp;
}

#[glib::derived_properties]
impl ObjectImpl for NscHeightClamp {
    fn constructed(&self) {
        self.parent_constructed();
        self.obj().set_overflow(gtk::Overflow::Hidden);
    }

    fn dispose(&self) {
        if let Some(child) = self.child.borrow_mut().take() {
            child.unparent();
        }
    }
}

impl WidgetImpl for NscHeightClamp {
    fn request_mode(&self) -> gtk::SizeRequestMode {
        gtk::SizeRequestMode::HeightForWidth
    }

    fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
        let Some(child) = self.child.borrow().clone() else {
            return (0, 0, -1, -1);
        };

        if orientation == gtk::Orientation::Vertical {
            let (child_min, child_nat, child_min_bl, child_nat_bl) =
                child.measure(gtk::Orientation::Vertical, for_size);

            let target = self.target_height(child_nat);
            let min = child_min.min(target);
            (min, target, child_min_bl, child_nat_bl)
        } else {
            child.measure(orientation, for_size)
        }
    }

    fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
        let Some(child) = self.child.borrow().clone() else {
            return;
        };

        let (_, child_natural, _, _) = child.measure(gtk::Orientation::Vertical, width);
        let child_height = child_natural.max(height);
        child.allocate(width, child_height, baseline, None);

        self.update_will_change(child_natural);
    }

    fn snapshot(&self, snapshot: &gtk::Snapshot) {
        let Some(child) = self.child.borrow().clone() else {
            return;
        };

        let widget = self.obj();
        let w = widget.width() as f32;
        let h = widget.height() as f32;
        let child_h = child.allocated_height() as f32;

        if child_h <= h || h <= 0.0 {
            widget.snapshot_child(&child, snapshot);
            return;
        }

        let overflow = child_h - h;
        let fade_strength = (overflow / FADE_HEIGHT).clamp(0.0, 1.0);
        let effective_fade = FADE_HEIGHT.min(h);
        let gradient_start = h - effective_fade;
        let bottom_alpha = if self.is_animating() && self.expanding.get() {
            1.0 - fade_strength
        } else {
            0.0
        };

        let bounds = graphene::Rect::new(0.0, 0.0, w, h);
        let start_frac = gradient_start / h;

        snapshot.push_mask(gsk::MaskMode::Alpha);

        snapshot.append_linear_gradient(
            &bounds,
            &graphene::Point::new(0.0, 0.0),
            &graphene::Point::new(0.0, h),
            &gsk::ColorStop::builder()
                .at(0.0, gdk::RGBA::new(0.0, 0.0, 0.0, 1.0))
                .at(start_frac, gdk::RGBA::new(0.0, 0.0, 0.0, 1.0))
                .at(1.0, gdk::RGBA::new(0.0, 0.0, 0.0, bottom_alpha))
                .build(),
        );

        snapshot.pop(); // end mask, begin source
        widget.snapshot_child(&child, snapshot);
        snapshot.pop(); // apply mask to source
    }
}
