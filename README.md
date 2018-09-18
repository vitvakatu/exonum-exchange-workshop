# Run App

```bash
cd app
rustup install nightly
rustup default nightly
cargo install cargo-web
cargo web build
cargo web start
```

Open http://localhost:8000 in your browser.

# Run Node

```bash
cd node
cargo build
cargo run -- run -c configs/node.toml -d db0 --public-api-address 127.0.0.1:8080
```
