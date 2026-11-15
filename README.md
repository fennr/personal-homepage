# personal-homepage

Личный сайт [fennr.ru](https://fennr.ru) на Rust: светлый документ для читателя
и терминальная версия по адресу `/terminal` — из одного и того же контента.

## Возможности

- Обычная версия: профиль, полоса карьеры, стек, опыт, проекты, контакты.
- Терминальная версия: `help`, `about`, `stack`, `work`, `projects`, `contact`,
  `resume`, `ls`, `cat <раздел>`, `clear` и подсказки-ссылки. Форма работает и
  без JS: без htmx она уходит обычным POST и возвращает целую страницу.
- Текст живёт в `content/*.toml`; правила проверяются на старте, а ошибка
  называет файл и поле.
- Шаблоны, стили, шрифты и htmx вкомпилированы в бинарь, статика отдаётся с
  `ETag` и сжатием.
- Сервер без состояния: без базы, сессий и cookies. Образ distroless — 6.7 МБ,
  работает с `read_only: true`.

## Быстрый старт

Нужны Rust 1.88+ и [just](https://github.com/casey/just).

```sh
just serve    # http://127.0.0.1:8080
```

Адрес меняется переменной `PERSONAL_HOMEPAGE_BIND`.

## Разработка

```sh
just check          # форматирование, clippy -D warnings, тесты
just content-check  # проверить content/ так же, как это сделает сервис
just fonts          # пересобрать шрифты (нужны fonttools и brotli)
just resume         # пересобрать PDF-резюме (нужен typst)
just image          # образ локально: personal-homepage:local
just release X.Y.Z  # версия, коммит, тег
just hooks          # хук commit-msg с commitlint (нужен node)
```

## Документация

- [Как устроено](docs/architecture.md) — режимы, маршруты, модули, статика
- [Контент](docs/content.md) — файлы, поля и правила проверки
- [Сборка и релиз](docs/release.md) — образ, CI, деплой в homelab
- [Решения](docs/decisions.md) — что выбрано и почему
