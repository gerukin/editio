# Editio

A small, fast terminal editor for the age of agents. They write the code.
You write the Markdown spec, revise the Markdown spec, and occasionally glance
at the code to find out which Markdown spec they read.

Editio makes that loop pleasant:

- **Markdown gets the good chair.** Rendered previews, diagrams, tables and a
  heading/link navigator. Switch between reading and editing without losing
  your place—or your train of prompt.
- **Only what you need.** Text editing, syntax highlighting, multi-cursors,
  search, JSON and diffs. No extension marketplace to develop a second career in.
- **Both hands welcome.** Keyboard and mouse work together: navigate, select,
  click and copy. Your mouse needn't resign when you open a terminal.
- **Shortcuts, not initiation rites.** Familiar VS Code-style editing and a
  searchable command palette. Save with Ctrl+S. Quit without a pilgrimage.
- **Autosaved recovery drafts.** Because shit happens. Unsaved edits get a
  checkpoint after 30 idle seconds, including untitled files, without overwriting
  the original. [The details, before you test gravity on your laptop.](docs/recovery.md)
- **Fast on its feet. Light on your RAM.** Built in Rust, with bounded work,
  cached rendering and an idle loop that actually takes the hint. Your agents
  have already called dibs on the rest of the machine.
- **Your terminal, your colors.** Follows its theme by default. Or choose the
  bundled Tokyo Night Omarchy theme, if you have taste. No judgment. Some judgment.

## Install

With [mise](https://mise.jdx.dev/) (including Omarchy/Arch):

```sh
mise use -g github:gerukin/editio@latest
```

mise may delay `@latest` for 24 hours after a release. To install this new release
immediately, use `mise use -g github:gerukin/editio@0.1.1`; switch back to `@latest`
after that window.

With Homebrew on macOS, using this same repository as the tap:

```sh
brew tap gerukin/editio https://github.com/gerukin/editio
brew install --cask gerukin/editio/editio
```

Or install **and update** on Linux/macOS with the same one-liner:

```sh
curl -fsSL https://github.com/gerukin/editio/releases/latest/download/install.sh | sh
```

**Windows:** download the matching ZIP from
[GitHub Releases](https://github.com/gerukin/editio/releases), extract it, and add
its directory to PATH.

Linux, macOS and Windows on x86-64 and ARM64 are targeted. Tested on the
developer's Linux x86-64 machine, ARM64 Macs and x86-64 Surface;
Linux ARM64, macOS x86-64 and Windows ARM64 remain untested.
macOS builds are not notarized; Windows builds are unsigned. Your OS may object
before you get a chance to object to the keyboard shortcuts.
See [installation, updates and requirements](docs/distribution.md).

## Put it to work

```sh
editio spec.md                        # Give the agents something to misinterpret
editio -v README.md                   # Read without the punctuation scaffolding
some-command | editio -v              # A pipe dream, delivered
editio --diff before.rs after.rs      # See what "small change" meant this time
```

**Ctrl+K** opens the command palette. **Ctrl+G** switches View/Edit,
**Ctrl+S** saves, **Ctrl+F** finds, and **Ctrl+Q** quits. Press **?** in View
for help, or run `editio --help` for CLI options.

## Read the fine Markdown

- [User guide and shortcuts](docs/guide.md) · [Examples](examples/README.md)
- [Configuration](docs/configuration.md) · [Recovery](docs/recovery.md)
- [Markdown navigation](docs/navigation.md) · [Diffs](docs/diffs.md)
- [Contributing and local development](CONTRIBUTING.md) · [Changelog](CHANGELOG.md)

Don't like it? Vibe code your own. Start with a good spec. We know an editor.

[MIT license](LICENSE) · Copyright © 2026 Nicolas Germineau
