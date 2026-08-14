# Contributing to tauri-browser-tabs

Thank you for your interest in contributing!

## Development Setup

1. **Prerequisites**
   - Node.js 18+
   - Rust 1.75+
   - Tauri prerequisites for your platform

2. **Clone and install**

   ```bash
   git clone https://github.com/your-org/tauri-browser-tabs.git
   cd tauri-browser-tabs
   npm install
   ```

3. **Build**

   ```bash
   npm run build
   ```

4. **Run example**

   ```bash
   npm run example:dev
   ```

## Project Structure

```
tauri-browser-tabs/
├── crates/
│   └── tauri-plugin-browser-tabs/    # Rust plugin
├── packages/
│   ├── core/                         # Framework-agnostic TS library
│   └── vue/                          # Vue 3 adapter
└── examples/
    └── basic/                        # Example Tauri app
```

## Making Changes

### Rust Plugin

- Source: `crates/tauri-plugin-browser-tabs/src/`
- Run `cargo check` in the crate directory to verify compilation.
- Platform-specific code goes in `src/platform/`.

### TypeScript Packages

- Core logic: `packages/core/src/`
- Vue adapter: `packages/vue/src/`
- Use `npm run build` to verify TypeScript compilation.

## Pull Request Process

1. Fork the repository.
2. Create a feature branch: `git checkout -b feature/my-feature`.
3. Make your changes and ensure they build.
4. Add or update tests if applicable.
5. Update documentation (README, doc comments).
6. Submit a pull request with a clear description.

## Code Style

- Rust: Follow `rustfmt` defaults.
- TypeScript: Follow the existing style (2 spaces, single quotes).
- Commit messages: Use conventional commits (e.g., `feat:`, `fix:`, `docs:`).

## Reporting Issues

Please include:

- Operating system and version
- Tauri version
- Steps to reproduce
- Expected vs actual behavior
- Relevant logs or screenshots

## License

By contributing, you agree that your contributions will be licensed under the same MIT OR Apache-2.0 dual license as the project.
