# Stage 1: Build
FROM rust AS builder

# Set the working directory inside the container
WORKDIR /usr/src/app/mock_rpc

# Copy only Cargo files first (to leverage Docker caching)
COPY mock_rpc/Cargo.toml mock_rpc/Cargo.lock ./

RUN cargo fetch

# Copy the rest of the source code
COPY mock_rpc .

# Compile the Rust application
RUN cargo build

# Stage 2: Runtime
FROM rust

# Set working directory in the final container
WORKDIR /usr/src/app/mock_rpc

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/app/mock_rpc/target/debug/mock_rpc .
# Copy mock data
COPY mock_rpc/mock_data ./mock_data

# Expose the mock_rpc service ports (should match docker-compose.yml)
EXPOSE 8545 8546

# Run the compiled Rust backend
CMD ["./mock_rpc"]