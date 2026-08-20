# shilpo-macros

Generic procedural macro tooling used by [Shilpo UI](https://github.com/shilpo-rs/ui) design-system
crates — not tied to any one design system.

- `icon_named!` — generates a `PascalCase` icon enum from a directory (or manifest file) of SVGs.
- `#[derive(IntoPlot)]` — generates a `gpui::IntoElement` implementation for chart types.

Currently used by [`shilpo-m3e`](../m3e); any future design-system crate in this repo can use it too.
