# Contributing

Forks, pull requests, bug reports and documentation fixes are welcome. This project is for the Conquer community to learn from and build on.

## Send a change

1. Fork the repository and create a branch for your change.
2. Keep the change focused. Open an issue before starting a large feature or rewrite.
3. Test it, then open a pull request against `main`. Explain what changed and how you tested it. Include a screenshot or short clip for visual changes.

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

For changes that affect the running client, launch it with your own 5065 files and check the result. Build instructions are in the [README](README.md).

## Keep it a skeleton

Rendering, movement, collision, file readers, minimap improvements and better tests are all useful contributions. Keep the base client small and offline. Discuss larger systems before adding them here, or explore them in your own fork.

Don't commit game assets, credentials, build outputs or paths tied to your computer. Use the original 5065 behavior as a reference when fixing compatibility issues, and explain the evidence behind the change.

Only submit code you have the right to contribute. Contributions are under the project's [Apache 2.0 license](LICENSE). Retain applicable attribution notices and mark your changes as the license requires.

## Ask or report a bug

[Open an issue](https://github.com/deklol/Preservation-CO-Client/issues) with steps to reproduce the problem, what you expected and what happened. Include your OS and GPU for rendering problems. Remove personal information from logs before sharing them.

You can also find us on [Discord](https://discord.gg/CvKPXEHYRY) for development discussions, source snippets and beta testing the full Preservation CO client. Discord is optional; issues and pull requests are welcome on GitHub.
