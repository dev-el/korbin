# Korbin

Korbin is a scriptable text editor written in Rust with native performance. It uses [Ribir](https://github.com/RibirX/Ribir) for its user interface and [Roto](https://codeberg.org/NLnetLabs/roto) as its scripting language.

## Features

- **Vi-like Keybindings**: Supports normal, insert, visual, command, and search modes.
- **Embedded Scripting**: Configure and extend your editor using the Roto scripting language.
- **Ribir-based UI**: High-performance, declarative UI rendering.
- **Tree-sitter Support**: Fast, accurate, and context-aware code highlighting. 
- **Rope Text Structure**: Efficient handling of large text files.

## Installation

You can install Korbin with `cargo`:

```bash
cargo install korbin
```

The binary will be placed in `~/.cargo/bin` by default. Ensure this directory is in your `PATH` to launch Korbin from your terminal.

## Configuration

Korbin looks for its configuration file at `~/.config/korbin/config.roto`.

## License

This project is licensed under the Apache License, Version 2.0. See the [LICENSE](LICENSE) file for details.
