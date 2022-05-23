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