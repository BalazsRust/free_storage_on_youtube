FROM docker.io/library/rust:1-slim-bookworm
RUN apt-get update && apt-get install -y --no-install-recommends ffmpeg && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . . 
RUN cargo build --release
CMD ["./target/release/binarycoder"]
