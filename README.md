# Korbin

Korbin is a scriptable text editor written in Rust with native performance. It uses [Ribir](https://github.com/RibirX/Ribir) for its user interface and [Roto](https://codeberg.org/NLnetLabs/roto) as its scripting language.

## Features

- **Vi-like Keybindings**: Supports normal, insert, visual, command and search modes.
- **Embedded Scripting**: Configure and extend your editor using the Roto scripting language.
- **Ribir-based UI**: High-performance, declarative UI rendering.
- **Tree-sitter Support**: Fast, accurate, and context-aware code highlighting. 
- **Rope Text Structure**: Efficient handling of large text files.

## Build

To build Korbin from source, you need to have Rust installed. Clone the repository and build using Cargo:

```bash
git clone https://github.com/dev-el/korbin.git
cd korbin
cargo build --release
```

The binary will be located at `target/release/korbin`.

## Configuration

Korbin looks for its configuration file at `~/.config/korbin/config.roto`.

## License

This project is licensed under the Apache License, Version 2.0. See the [LICENSE](LICENSE) file for details.
