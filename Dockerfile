# Build stage
FROM rust:latest AS builder

WORKDIR /app

# Accept the build argument
ARG DATABASE_URL

# Make sure to use the ARG in ENV
ENV DATABASE_URL=$DATABASE_URL

# Copy the source code
COPY . .

# Build the application
RUN cargo build --release


# Production stage
FROM debian:stable-slim

WORKDIR /usr/local/bin

COPY --from=builder /app/target/release/blackjack-backend .

CMD ["./blackjack-backend"]

