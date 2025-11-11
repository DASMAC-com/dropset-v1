# `dropset` Workspace

`dropset` is an on-chain orderbook program built for Solana.

## 🧱 Workspace Overview

### **`dropset-interface`**
The program’s external interface: instruction schemas, account layouts, and
shared state types.  
This crate is client-agnostic and can be imported by both on-chain and off-chain
consumers.

### **`instruction-macros`**
A procedural macro crate that generates strongly-typed instruction builders,
account context structs, and validation scaffolding.

### **`instruction-macros-impl`**
The implementation crate for `instruction-macros`.  
Handles all the parsing, validation, and token generation logic for the
procedural macros.

### **`dropset-program`**
The Solana on-chain program.

### **`dropset-client`**
A lightweight Rust client for local testing, benchmarking, and integration with
RPC services.  
Provides helpers for sending transactions and fetching parsed state via the JSON
RPC API.

## 📚 Documentation

You can generate and open the full internal documentation (including private
items and doc-hidden modules) using:

```bash
cargo doc --open --no-deps --document-private-items
```

### 💻 Viewing on `wsl`

If you're on Windows Subsystem for Linux `cargo doc --open` won't automatically
work. You must convert the doc target path to a Windows path first, and then
manually open it in your browser.

If you're using `chrome.exe` for example, at the repository root you could run:

```bash
# Build the docs first
cargo doc --no-deps --document-private-items

# Set the docs path as a Windows path
DOCS_PATH=$(wslpath -w target/doc/dropset_program/index.html)

# Open the docs in Chrome
chrome.exe "$DOCS_PATH"

# ---
# As a copy pastable one-liner
cargo doc --no-deps --document-private-items && \
  chrome.exe "$(wslpath -w target/doc/dropset_program/index.html)"
```
