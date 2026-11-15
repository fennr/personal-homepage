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

# Поднять страницу локально на 127.0.0.1:8080 (порт — PERSONAL_HOMEPAGE_BIND).
serve:
    cargo run

# Проверить контент из content/ так же, как это сделает сервис на старте.
content-check:
    cargo run --quiet

# Пересобрать шрифты: нарезка по алфавитам (нужны python3, fonttools и brotli).
fonts:
    python3 -m venv .venv-fonts
    .venv-fonts/bin/pip install --quiet --upgrade fonttools brotli
    .venv-fonts/bin/python scripts/fonts

# Пересобрать PDF-резюме: данные из бинаря, вёрстка в Typst (нужен typst).
resume:
    cargo run --quiet -- resume > resume/.data.json
    mv resume/.data.json resume/data.json
    typst compile --font-path resume/fonts resume/resume.typ static/resume.pdf
    @echo "static/resume.pdf обновлён"

# Собрать образ локально, без публикации.
image:
    docker build --tag personal-homepage:local .

# Установить хук commit-msg с commitlint: нужен node, в CI он не участвует.
hooks:
    npm install --no-audit --no-fund
    install -m 0755 scripts/commit-msg .git/hooks/commit-msg
    @echo "хук commit-msg установлен"

# Выпустить версию X.Y.Z: поднять версию, поставить тег, напомнить про push.
release version:
    #!/usr/bin/env bash
    set -euo pipefail

    version={{quote(version)}}
    if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
      echo "версия должна быть вида X.Y.Z" >&2
      exit 1
    fi
    if [[ -n "$(git status --porcelain)" ]]; then
      echo "рабочий каталог должен быть чистым" >&2
      exit 1
    fi
    if git rev-parse --quiet --verify "refs/tags/v$version" >/dev/null; then
      echo "тег v$version уже существует" >&2
      exit 1
    fi

    current=$(python3 -c 'import pathlib, tomllib; print(tomllib.loads(pathlib.Path("Cargo.toml").read_text())["package"]["version"])')
    if [[ "$version" == "$current" ]]; then
      echo "версия $version уже в Cargo.toml: только тег"
    else
      python3 -c '
    import pathlib
    import sys

    version = sys.argv[1]
    path = pathlib.Path("Cargo.toml")
    lines = path.read_text().splitlines(keepends=True)
    for index, line in enumerate(lines):
        if line.startswith("version = \""):
            lines[index] = f"version = \"{version}\"\n"
            break
    else:
        raise SystemExit("не нашёл строку version в Cargo.toml")
    path.write_text("".join(lines))
    ' "$version"
      just check
      git add Cargo.toml Cargo.lock
      git commit -m "chore(release): $version"
    fi

    git tag --annotate "v$version" --message "v$version"
    echo "создан тег v$version"
    echo "публикация: git push origin HEAD && git push origin v$version"
