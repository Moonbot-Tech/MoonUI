# Longbridge GPUI Component donor source

This directory contains the pinned `crates/ui/src` source snapshot used by
`cargo xtask component-mirror` as the donor side for Mirror components.

- upstream: https://github.com/longbridge/gpui-component
- commit: cda0fc7fd4e4809dd2a8bae0337ac43b49e9f675
- source root: crates/ui/src

Do not edit this directory by hand. Refresh it only when intentionally bumping
the Longbridge donor pin and then update `component-manifest.json` plus
`docs/component-mirror-baseline.json` in the same change.
