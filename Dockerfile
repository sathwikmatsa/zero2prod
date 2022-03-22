# builder stage
FROM rust:1.59.0 AS builder
WORKDIR /app
COPY . .
COPY configuration configuration
ENV SQLX_OFFLINE true
# Build our application, leveraging the cac
RUN cargo install --path .

# runtime stage
FROM debian:bullseye-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
&& apt-get install -y --no-install-recommends openssl ca-certificates \
&& apt-get autoremove -y \
&& apt-get clean -y \
&& rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/zero2prod zero2prod
COPY configuration configuration
ENV APP_ENVIRONMENT production