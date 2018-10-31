FROM clux/muslrust:nightly as builder

RUN mkdir /home/rust
WORKDIR /home/rust

# Avoid having to install/build all dependencies by copying
# the Cargo files and making a dummy src/main.rs
COPY Cargo.toml .
COPY Cargo.lock .
RUN mkdir src
RUN echo "fn main() {}" > src/main.rs
RUN cargo test
RUN cargo build --release

# We need to touch our real main.rs file or else docker will use
# the cached one.
COPY . .
RUN touch src/main.rs

RUN cargo test
RUN cargo build --release

# Size optimization
RUN strip target/x86_64-unknown-linux-musl/release/soxy

# Start building the final image
FROM alpine
WORKDIR /home/rust
COPY --from=builder /home/rust/target/x86_64-unknown-linux-musl/release/soxy .
ENTRYPOINT ["./soxy"]
