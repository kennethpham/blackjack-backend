# Build stage
FROM rust:latest

WORKDIR /app

# Accept the build argument
ARG DATABASE_URL

# Make sure to use the ARG in ENV
ENV DATABASE_URL=$DATABASE_URL

# Copy the source code
COPY target/release/blackjack-backend .

CMD ["./blackjack-backend"]
