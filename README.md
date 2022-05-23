[![Rust](https://github.com/Oyelowo/payment_engine/actions/workflows/check_code.yml/badge.svg?branch=master)](https://github.com/Oyelowo/payment_engine/actions/workflows/check_code.yml)

# Description 
Toy payment engine
# How to run

```rs
cargo run -- transactions.csv > accounts.csv
```

## How to build

```rs
cargo build --release
```

## Documentation
```rs
cargo doc --open
```

## File structure
```
.
├── features
│   ├── account.rs
│   ├── mod.rs
│   ├── store.rs
│   └── transaction.rs
└── main.rs
```
