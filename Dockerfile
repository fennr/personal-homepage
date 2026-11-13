# syntax=docker/dockerfile:1

# Сборка в alpine: это musl-дистрибутив, поэтому бинарь сразу получается
# статическим и запускается в distroless, где нет ни libc-обвязки, ни shell.
# Целевой триплет отдельно не задаётся: x86_64-unknown-linux-musl здесь родной.
FROM rust:1.99-alpine AS build
WORKDIR /build

RUN apk add --no-cache musl-dev

# Слой зависимостей отдельно от кода: правка src не пересобирает все крейты.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && printf 'fn main() {}\n' > src/main.rs \
    && printf '\n' > src/lib.rs \
    && cargo build --locked --release \
    && rm -rf src

# Контент, шаблоны и статика вкомпилированы в бинарь, поэтому копируются до
# сборки, а не в финальный слой.
COPY content ./content
COPY templates ./templates
COPY static ./static
COPY src ./src

RUN touch src/main.rs src/lib.rs && cargo build --locked --release

FROM gcr.io/distroless/static-debian12:nonroot
WORKDIR /app

COPY --from=build /build/target/release/personal-homepage ./

# Адрес задаётся в образе, чтобы контейнер поднимался без переменных окружения.
ENV PERSONAL_HOMEPAGE_BIND=0.0.0.0:8080
EXPOSE 8080

# В distroless нет curl: проверку делает сам бинарь подкомандой health.
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/app/personal-homepage", "health"]

USER nonroot:nonroot
ENTRYPOINT ["/app/personal-homepage"]
