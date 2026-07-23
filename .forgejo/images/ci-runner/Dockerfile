FROM ghcr.io/catthehacker/ubuntu:act-24.04

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    clang-18 \
    libclang-rt-18-dev \
    libvulkan1 && \
    rm -rf /var/lib/apt/lists/*
