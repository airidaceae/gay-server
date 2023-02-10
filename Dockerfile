# Tells docker to use the latest Rust official image
FROM rust:latest AS base

# Copy our current working directory into the container
WORKDIR /app
COPY ./ /app

# Create the release build for musl with static libraries (common glibc L)
RUN rustup target add x86_64-unknown-linux-musl
RUN RUSTFLAGS='-C link-arg=-s' cargo build --release --target x86_64-unknown-linux-musl

# make a stage for the runner
FROM alpine:latest AS runner

# Change workdir and copy binary
WORKDIR /app
COPY --from=base /app/target/x86_64-unknown-linux-musl/release/gay-server /app/

# set up the www volume
VOLUME /app/www

# Expose the port it runs on
EXPOSE 12345

# add and switch user so we dont run as root
RUN adduser -u 5678 --disabled-password --gecos "" serveruser
USER serveruser

# Run the server
CMD ["/app/gay-server", "12345"]
