use std::ops::Deref;

use gpui::{App, SharedString};

pub mod controls;
pub mod foundation;
pub mod layout;
pub mod navigation;

pub mod alert;
pub mod avatar;
pub mod badge;
pub mod chart;
pub mod description_list;
pub mod dialog;
pub mod floating_toolbar;
pub mod hover_card;
pub mod kbd;
pub mod label;
pub mod list;
pub mod menu;
pub mod native_menu;
pub mod notification;
pub mod plot;
pub mod popover;
pub mod progress;
pub mod rating;
pub mod searchable_list;
pub mod sheet;
pub mod skeleton;
pub mod table;
pub mod tag;
pub mod tooltip;
pub mod tree;

pub use foundation::element_ext::*;
pub use foundation::event::InteractiveElementExt;
pub use foundation::focus_trap::{FocusTrapContainer, FocusTrapElement};
pub use foundation::geometry::*;
pub use foundation::global_state::GlobalState;
pub use foundation::icon::*;
pub use foundation::index_path::IndexPath;
pub use controls::input::{Rope, RopeExt, RopeLines};
pub use navigation::navigation_rail::{
    NavigationRail, NavigationRailArrangement, NavigationRailFooter, NavigationRailHeader,
    NavigationRailItem, NavigationRailMenuButton,
};
pub use layout::side_panel::{SidePanel, SidePanelPosition};

pub use crate::Disableable;

pub use foundation::font::FontFamilyCache;
pub use foundation::i18n::LocaleCatalogue;
#[cfg(any(feature = "inspector", debug_assertions))]
pub use foundation::inspector::*;
pub use menu::{ContextMenu, ContextMenuExt, ContextMenuState, PopupMenu, PopupMenuItem};
pub use foundation::root::Root;
pub use shilpo_macros::icon_named;
pub use foundation::styled::*;
pub use foundation::theme::*;
pub use controls::time::{calendar, date_picker};
pub use navigation::title_bar::*;
pub use foundation::virtual_list::{VirtualList, VirtualListScrollHandle, h_virtual_list, v_virtual_list};
pub use foundation::window_border::{WindowBorder, window_border, window_paddings};
pub use foundation::window_ext::WindowExt;

rust_i18n::i18n!("locales", fallback = "en");

/// Initialize the components.
///
/// You must initialize the components at your application's entry point.
pub fn init(cx: &mut App) {
    foundation::theme::init(cx);
    foundation::font::FontFamilyCache::init_global(cx);
    foundation::global_state::init(cx);
    #[cfg(any(feature = "inspector", debug_assertions))]
    foundation::inspector::init(cx);
    foundation::root::init(cx);
    foundation::focus_trap::init(cx);
    controls::color_picker::init(cx);
    controls::time::date_picker::init(cx);
    layout::dock::init(cx);
    sheet::init(cx);
    controls::combobox::init(cx);
    controls::select::init(cx);
    controls::input::init(cx);
    list::init(cx);
    dialog::init(cx);
    popover::init(cx);
    menu::init(cx);
    table::init(cx);
    foundation::text::init(cx);
    tree::init(cx);
    tooltip::init(cx);
}

#[inline]
pub fn locale() -> impl Deref<Target = str> {
    rust_i18n::locale()
}

#[inline]
pub fn set_locale(locale: &str) {
    rust_i18n::set_locale(locale)
}

#[inline]
pub(crate) fn measure_enable() -> bool {
    std::env::var("ZED_MEASUREMENTS").is_ok() || std::env::var("GPUI_MEASUREMENTS").is_ok()
}

/// Measures the execution time of a function and logs it if `if_` is true.
///
/// And need env `GPUI_MEASUREMENTS=1`
#[inline]
#[track_caller]
pub fn measure_if(name: impl Into<SharedString>, if_: bool, f: impl FnOnce()) {
    if if_ && measure_enable() {
        let measure = Measure::new(name);
        f();
        measure.end();
    } else {
        f();
    }
}

/// Measures the execution time.
#[inline]
#[track_caller]
pub fn measure(name: impl Into<SharedString>, f: impl FnOnce()) {
    measure_if(name, true, f);
}

pub struct Measure {
    name: SharedString,
    start: std::time::Instant,
}

impl Measure {
    #[track_caller]
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            start: std::time::Instant::now(),
        }
    }

    #[track_caller]
    pub fn end(self) {
        let duration = self.start.elapsed();
        tracing::trace!("{} in {:?}", self.name, duration);
    }
}
