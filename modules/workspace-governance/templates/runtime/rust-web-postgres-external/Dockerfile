FROM rust:1.85-bookworm AS build
ARG APP_PROJECT={{app_project}}
WORKDIR /src
COPY . .
RUN cargo build --release --locked --manifest-path "$APP_PROJECT/Cargo.toml"

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 appuser
COPY --from=build /src/target/release/{{project_name}} /app/{{project_name}}
USER appuser
ENV APP_HOST=0.0.0.0
ENV APP_PORT=8080
EXPOSE 8080
ENTRYPOINT ["/app/{{project_name}}"]
