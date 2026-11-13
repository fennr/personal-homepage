# Задачи проекта. Всё, что нужно перед коммитом и релизом, — здесь.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    @just --list

# Гейт качества: форматирование, линт и тесты. Именно это должно быть зелёным.
check:
    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo test

# Переформатировать код.
fmt:
    cargo fmt --all

# Проверить контент из content/ так же, как это сделает сервис на старте.
content-check:
    cargo run --quiet

# Установить хук commit-msg с commitlint: нужен node, в CI он не участвует.
hooks:
    npm install --no-audit --no-fund
    install -m 0755 scripts/commit-msg .git/hooks/commit-msg
    @echo "хук commit-msg установлен"
