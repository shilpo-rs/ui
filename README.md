# Shilpo UI

Cross-platform desktop UI, built on [GPUI](https://github.com/zed-industries/zed), for the
[Shilpo](https://github.com/shilpo-rs/shilpo) desktop environment and any other GPUI application.

This repo is a home for a **family of design systems**, not a single UI library. Today it holds one:

| Crate | Description | Directory |
|:--|:--|:--|
| **`shilpo-m3e`** | Material Design 3 (Expressive) components | [`m3e`](m3e) |
| **`shilpo-theme`** | M3 color math & data types | [`theme`](theme) |
| **`shilpo-macros`** | Procedural macros (icon generation, plot derive) | [`macros`](macros) |
| **`storybook`** | Interactive gallery demoing the component librar(y/ies) | [`storybook`](storybook) |

More design systems (WinUI-, GNOME-, or other-inspired) are expected to land here the same way, each as its own
crate alongside `m3e`, sharing the generic proc-macro tooling in `macros` but bringing their own theme/color math
where the design language actually differs (see `theme`'s M3-specific palette generation, which a different design
system would not reuse as-is).

## Building

```bash
cargo build --workspace
```

`storybook` is cross-platform and has no dependency on the Shilpo desktop shell; run it directly with `cargo run -p storybook`.

## Acknowledgements & Prior Art

This UI library started as a fork of [`gpui-component`](https://github.com/longbridge/gpui-component). We extend our
deep gratitude to the original authors and maintainers of `gpui-component` for creating a fantastic foundation.

It has since evolved with extensive modifications, including Material Design 3 / Material Expressive design tokens,
customized layout physics, desktop notification integrations, and tailored component styling.

> **Disclaimer**: This project is independent and open-source, and is **not affiliated with, endorsed by, or
> supported by Google or the `gpui-component` maintainers in any way.**
