# ECDSA & Kirk Engine Reverse Engineering (PSP Cryptography)

A clean-room translation and reverse-engineering project of Sony PSP cryptographic routines (specifically Elliptic Curve Digital Signature Algorithm and Kirk engine command 12/14 primitives) from legacy C to safe, modern **Rust**.

---

## Project Overview

This repository documents the process of breaking down, analyzing, and porting legacy C cryptographic implementations used in PSP firmware into a structured, memory-safe Rust workspace. 

The primary goals of this project are:
1. **Isolate and Document:** Extract core cryptographic C files (`ec.c`, `bn.c`, `kirk_engine.c`) and test harnesses to understand low-level operations.
2. **Translate to Rust:** Reimplement cryptographic routines (such as point addition, point doubling, and scalar multiplication) utilizing `num-bigint` for arbitrary precision arithmetic.

---

## Repository Structure

```text
.
├── c_file/                 # Isolated legacy C implementation and test harnesses
│   ├── AES.c / .h          # AES encryption routines
│   ├── SHA1.c / .h         # SHA-1 hashing implementation
│   ├── bn.c                # Big Number arithmetic routines
│   ├── ec.c                # Elliptic Curve arithmetic and point operations
│   ├── kirk_engine.c / .h  # Kirk firmware command engine (CMD12/CMD14)
│   ├── main.c              # C verification test harness
│   └── c_test              # Compiled C test binary
│
├── ecdsa_experiment/       # Rust experimental translation workspace
│   ├── Cargo.toml          # Rust dependencies (num-bigint, num-traits)
│   ├── TODO.md             # Active feature and translation checklist
│   └── src/
│       ├── main.rs         # Rust test execution and entry point
│       ├── kirk_headers.rs # Protocol headers and struct layouts
│       └── misc_decrypt/   # Core crypto engine modules
│           ├── mod.rs
│           ├── ecdsa_engine.rs  # EC point arithmetic & scalar multiplication
│           └── mod_operations.rs# Modular arithmetic helpers
│
└── research_notes/         # Deep-dive documentation and technical analysis
    └── reverse-engineering-kirk-cmd12.md
```
