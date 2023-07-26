FROM rust:1-alpine

# Override location for Miri sysroot since default /root/.cache is inaccessible to Jenkins Docker user
ENV MIRI_SYSROOT=/usr/local/miri_sysroot

# Add build dependencies for Impact
RUN apk add --no-cache build-base cmake pkgconfig fontconfig-dev

# Install Miri
RUN rustup toolchain install nightly --component miri

# Create sysroot for Miri at $MIRI_SYSROOT
RUN cargo +nightly miri setup

# Ensure everything in $RUSTUP_HOME and $CARGO_HOME is writable by Jenkins Docker user
RUN chmod -R a+w $RUSTUP_HOME $CARGO_HOME