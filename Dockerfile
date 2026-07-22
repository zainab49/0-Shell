# ---- Build stage -----------------------------------------------------------
# Compiles the shell with the official Rust toolchain so nothing needs to be
# installed on the host machine.
FROM rust:1.82-slim AS builder

WORKDIR /app

# Copy the manifest first and pre-fetch dependencies so this layer is cached
# across source-only changes. (This project has no external dependencies, but
# the pattern keeps rebuilds fast if any are added later.)
COPY Cargo.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

# Now copy the real sources and build the release binary.
COPY src ./src
RUN touch src/main.rs && cargo build --release

# ---- Runtime stage ---------------------------------------------------------
# A minimal Debian image that carries /etc/passwd and /etc/group, which `ls -l`
# reads to resolve owner and group names.
FROM debian:bookworm-slim AS runtime

WORKDIR /root

# Copy just the compiled binary from the build stage.
COPY --from=builder /app/target/release/0-shell /usr/local/bin/0-shell

# Launch the shell by default. Run the container interactively (docker run -it).
ENTRYPOINT ["0-shell"]
