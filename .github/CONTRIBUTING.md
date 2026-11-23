# Contributing to HaloCord for Yapfast

Thanks for helping. Bug reports, ideas and pull requests are all welcome.

## Reporting a bug

[Open an issue](https://github.com/DavidHiFi/HaloCord-Yapfast/issues/new) and include:

- what you did, what you expected and what happened instead
- your Windows version and the Yapfast version it was built on
- a screenshot of the HaloCord page if it looks wrong

Problems that also happen in stock Yapfast belong with [Yapfast](https://github.com/yapfast/yapfast/issues).

## Suggesting a plugin

Open an issue that says what the plugin should do and why. If it is a HaloCord or TestCord plugin you miss, name it.

## Sending a pull request

1. Fork the repo and create a branch from `halocord`.
2. Build it as the README shows.
3. Run the tests before you push: `cargo test --locked --no-default-features --features desktop,discord`.
4. Keep each pull request to one change, and say in the description how you tested it.

HaloCord's code lives in its own `halocord.rs` files. A new plugin is one entry in `PLUGINS` in `crates/yapfast-iced/src/halocord.rs`. Keep changes to Yapfast's own files as small hooks, and list each one in `docs/halocord.md`.

## License

By contributing, you agree that your work is released under this repo's [GNU General Public License](../LICENSE), version 3 or later.
