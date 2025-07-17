# emacs-solidity-server

`emacs-solidity-server` is a fast Solidity language server written in Rust. It’s built for Emacs users who want precise go-to-definition and seamless remapping support without the bloat of a full IDE.

> Two things to note:<br>
> First one is: it does just enough, and does it right.<br>
> The other one is: it goes real well with Doom Emacs.

If you're an auditor or developer working on real-world Solidity codebases, this tool is designed to reduce friction and let you stay keyboard-driven.

---

## Why Another LSP?

Most existing Solidity LSPs either break under real audit setups or bury you in unnecessary features. This one sticks to the essentials.

---

## Feature Highlights (Current)

* Go-to-definition via native `solc` AST traversal
* Diagnostics directly from `solc` compiler
* Pragma-aware version resolution with persistent caching
* Import remapping with support for common layouts
* Works out of the box with **Foundry** and **Hardhat**
* Written in safe Rust with minimal runtime dependencies

> Use it alongside [`solidity-mode`](https://github.com/ethereum/emacs-solidity) for syntax highlighting and tight Emacs integration.

---

## Installation

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Clone and Build

```bash
git clone https://github.com/0xh4ty/emacs-solidity-server.git
cd emacs-solidity-server
cargo build --release
```

The binary will be located at:

```bash
target/release/emacs-solidity-server
```

---

## Usage Notes

1. **Pragma-Aware Compilation**
   The server parses the first `pragma solidity` directive in each file and fetches the latest matching patch version. Binaries are cached under `~/.cache/emacs-solidity-server/solc/`.

2. **Import Remappings**
   Recognizes remapping formats from:

   * `remappings.txt`
   * `foundry.toml`
   * `hardhat.config.js/ts`
   * `truffle-config.js`

3. **First-Run Compiler Downloads**
   Ensure internet access during first use. The server will download `solc` binaries as needed.

4. **Syntax and Diagnostics**
   Uses `solc` for diagnostics. Use `solidity-mode` for syntax highlighting and disable Flycheck to prevent conflicts.

---

## Emacs Configuration (Eglot)

```elisp
(with-eval-after-load 'solidity-mode
  ;; Disable Flycheck. Diagnostics come from solc directly
  (advice-add 'solidity-flycheck-setup :override #'ignore))

(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               '(solidity-mode . ("~/path/to/emacs-solidity-server")))  ;; Adjust path
  (add-hook 'solidity-mode-hook #'eglot-ensure))
```

---

## Features in Development

* Neovim support
* Hover info and auto-completion
* Enhanced syntax integration
* Support for Truffle and Brownie project structures

---

## Issues and Feature Requests

If you spot a bug, hit something broken, or want to request a feature, open an issue.
This project is focused and minimal by design, but if something is genuinely useful for audit workflows, it might get in.

[→ Open an issue](https://github.com/0xh4ty/emacs-solidity-server/issues)

---

## Contributions

Contributions are welcome. Fork the repo, make your changes, and open a pull request.
If you're fixing remapping edge cases, compiler version handling, or project compatibility quirks, even better.

---

## License

This project is licensed under the MIT License.

---

Developed by [0xh4ty](https://x.com/0xh4ty) for developers and auditors who prefer keyboard-driven workflows and compiler-accurate navigation.
