# ECDSA & Kirk Engine Reverse Engineering (PSP Cryptography)

A clean-room translation and reverse-engineering project of Sony PSP cryptographic routines (specifically Elliptic Curve Digital Signature Algorithm and Kirk engine command 12/14 primitives) from legacy C to safe, modern **Rust**.

---

## Project Overview

This repository documents the ongoing process of breaking down, analyzing, and porting legacy C cryptographic implementations used in PSP firmware into a structured, memory-safe Rust workspace. 

The primary goals of this project are:
1. **Isolate and Document:** Extract core cryptographic C files (`ec.c`, `bn.c`, `kirk_engine.c`) and test harnesses to understand low-level operations.
2. **Translate to Rust:** Reimplement cryptographic routines (such as point addition, point doubling, and scalar multiplication) utilizing `num-bigint` for arbitrary precision arithmetic.
3. **Bridge the Gap:** Understand architectural decisions made by legacy developers—such as pointer-casting hacks, output pointer patterns, and Montgomery domain optimizations—and adapt them into idiomatic, safe Rust.

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

---

## Key Technical Findings & Insights

### 1. Handling 160-Bit Cryptography Without Native Hardware Support
The legacy PSP processor lacked native 160-bit arithmetic registers. Consequently, the C implementation utilized byte arrays (`u8` pointers) combined with manual looping (`bn_` routines) to simulate precision arithmetic. 
* **The Rust Approach:** We replaced these manual C arrays with the `num-bigint` crate (`BigUint`), leveraging dynamically allocated heap vectors to handle multi-precision math safely.

### 2. The Legacy Pointer Casting Hack (`ec_priv_to_pub`)
In the original C codebase, functions like `ec_priv_to_pub` pass raw `u8*` pointers to struct fields (such as `public_key.x`) knowing that `y` sits immediately adjacent in memory:
```c
typedef struct {
    u8 x[0x14]; // Bytes 0 to 19
    u8 y[0x14]; // Bytes 20 to 39
} ECDSA_POINT;
```
This allowed functions writing 40 bytes of coordinate data to seamlessly overflow from `x` into `y` for performance optimization. In our Rust implementation, we handle coordinate writing safely through structured references to both axes.

### 3. Montgomery Domain Optimization
To avoid expensive modular long division (`% P`) during hundreds of point additions and doublings, the legacy engine maps coordinates into the Montgomery domain using a power-of-2 multiplier ($R$). 
* While the C engine relies heavily on `bn_to_mon` and `bn_from_mon` to exploit fast bit-shifting (`>> 1`), Rust's high-performance bigint operations allow us to keep code clean and readable, though verifying arithmetic parity against C requires strict alignment of curve parameters (such as the prime field `EC_P`).

### 4. Curve Parameters & The Borrow Checker Architecture
* **$P$ (Prime Field):** Defines the geometric ceiling for curve coordinates ($X$ and $Y$).
* **$N$ (Curve Order):** Defines the total number of valid points and bounds the private scalar key.
* **Rust Mutability:** To satisfy Rust's strict borrow checker during the *Double-and-Add* loop (`point_mul`), arithmetic operations that mutate points in-place are carefully separated into dedicated helpers (`point_double_in_place` vs. `point_double_out_place`).

---

## Verification & Testing

To ensure mathematical accuracy during translation, atomic unit tests are run in parallel against both the isolated C test binary (`c_test`) and the Rust test runner:

1. **C Verification:**
   ```bash
   cd c_file/
   gcc main.c ec.c bn.c kirk_engine.c SHA1.c AES.c -o c_test
   ./c_test
   ```
2. **Rust Verification:**
   ```bash
   cd ecdsa_experiment/
   cargo test
   ```

---

## License & Disclaimer
This project is intended strictly for educational, research, and archival purposes regarding consumer electronics reverse engineering.