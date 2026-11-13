# Сборка и релиз

## Образ

Две ступени: `rust:1.99-alpine` (musl) собирает статический бинарь, он уезжает
в `gcr.io/distroless/static-debian12:nonroot`. Shell и curl внутри нет, поэтому
`HEALTHCHECK` вызывает сам бинарь: `personal-homepage health` делает
`GET /healthz` на локальный порт и возвращает код выхода. Контейнер работает с
`read_only: true`; адрес задаётся переменной `PERSONAL_HOMEPAGE_BIND` (в образе
— `0.0.0.0:8080`).

```sh
just image    # personal-homepage:local, 6.7 МБ
```

## CI

Workflow `Container` на каждый push и pull request: job `check` (fmt, clippy,
тесты) и job `build`. Публикация в `ghcr.io/fennr/personal-homepage` — только по
тегу `vX.Y.Z`, и тег обязан совпадать с версией в `Cargo.toml`. Пакет читается
анонимно, поэтому на сервере `docker login` не нужен.

## Релиз

```sh
just release 0.2.2    # версия в Cargo.toml, коммит, аннотированный тег
git push origin HEAD && git push origin v0.2.2
```

`just release` не выпустит версию из грязного каталога и не перезапишет
существующий тег. Номер: patch — тексты и мелкие правки, minor — новые поля
контента или поведение.

Сообщения коммитов — Conventional Commits, трейлер `Co-authored-by` запрещён.
Версия проекта живёт в `Cargo.toml`; node нужен только для хука `commit-msg`,
в CI он не участвует.

## Деплой

Compose-файла в этом репозитории нет: сервис описан в репозитории `homelab`
(`services/personal-homepage`), туда же пинится тег образа. Обновление — правка
тега, `make lint`, коммит и push, затем в `homelab`:

```sh
make remote-deploy    # таймер применит изменения и сам, но не сразу
make remote-status    # контейнер healthy
make remote-drift     # «расхождений нет»
```
