# shilpo-m3e

Material Design 3 (Expressive) desktop UI components for [GPUI](https://github.com/zed-industries/zed).

`m3e` — Material 3 Expressive — is one design system in the [Shilpo UI](https://github.com/shilpo-rs/ui) family.
It's the default UI library used by the [Shilpo](https://github.com/shilpo-rs/shilpo) desktop environment, but has
no dependency on it — any GPUI application can use `shilpo-m3e` on its own.

## Usage

```toml
[dependencies]
shilpo-m3e = "0.1"
```

```rust
shilpo_m3e::init(cx);
```

See [`storybook`](../storybook) in this repo for a full interactive gallery of every component.

## Acknowledgements

This library started as a fork of [`gpui-component`](https://github.com/longbridge/gpui-component); see the
[repo README](../README.md#acknowledgements--prior-art) for full attribution.
