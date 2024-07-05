# Full stack web app

*Using Leptos SSR, Axum & Tailwind*

## Requirement

```bash
cargo install cargo-leptos
```

## Usage

```bash
cargo leptos watch
```

If the server don't build run before:  

```bash
cargo build -p server
```

## Tests

### Unit tests

For testing component logic and server functions:
(*Components logic are tested by reproducing the logic into an unit test*)

```bash
cargo test
```

### End to end tests

Not working so far

```bash
wasm-pack test --chrome app/
```
