# Use a base Linux image
FROM ubuntu:latest

# Install necessary dependencies
RUN apt update && apt install -y \
    git \
    curl \
    ffmpeg \
    npm \
    pkg-config \
    nodejs \
    && apt clean

# Clone the repository
RUN git clone https://github.com/hsa00000/Urocissa /Urocissa

# Set working directory
WORKDIR /Urocissa

# Install Rust using rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Set environment variable for Rust manually and run the script
ENV PATH="/root/.cargo/bin:${PATH}"
RUN node install-urocissa.mjs

# Default command
CMD ["node", "run-urocissa.mjs"]
