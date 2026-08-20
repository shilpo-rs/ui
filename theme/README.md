# shilpo-theme

Material Design 3 color math and theme state: seed color → M3 dynamic-color scheme generation via
`mcu_material_color`, plus the `ThemeMode`/`ColorSource`/`SchemeVariant`/`ThemeState` reducer shape.

Used by [`shilpo-m3e`](../m3e), which owns the actual component rendering. This crate is pure
computation with zero system dependencies — see [the repo README](../README.md) for how it fits into
the broader [Shilpo UI](https://github.com/shilpo-rs/ui) family.
