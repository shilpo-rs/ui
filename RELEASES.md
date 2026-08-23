# UI release contract

The UI repository has an independent release cycle from the Shilpo product, SDKs, and extension registry. Initially,
`shilpo-m3e`, `shilpo-theme`, and `shilpo-macros` are a synchronized release trio: all three manifest versions must be
identical and one `ui-vX.Y.Z` tag releases that version. `storybook` is an internal validation application and is not
part of the published trio.

The organization-wide namespaces are `shilpo-vX.Y.Z` for the product, `ui-vX.Y.Z` for this trio,
`rust-sdk-vX.Y.Z`/`typescript-sdk-vX.Y.Z` for SDKs, and `<extension-id>-vX.Y.Z` for an individual extension. Existing
migrated tags remain untouched and are not reinterpreted.

## Changelog policy

Pull-request titles use Conventional Commit style; local commits need not. Squash merges preserve the validated title
on `main`. `git-cliff` 2.13.1 plus `cliff.toml` is the deterministic release-note contract. Historical unconventional
commits are retained under **Other changes**. Run `scripts/release-contract.sh dry-run` to compare two generated note
files byte for byte.

## Tag preflight

Before publication, set the same semantic version in `m3e/Cargo.toml`, `theme/Cargo.toml`, and `macros/Cargo.toml`,
merge it, and create `ui-vX.Y.Z` on that commit. From a clean checkout of the tag, run
`scripts/release-contract.sh validate-tag ui-vX.Y.Z`. The command rejects a dirty tree, a tag missing from the fetched
`origin/main` ancestry, different trio versions, and a tag/version mismatch. It only validates; it never changes tags.
