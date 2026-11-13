// Резюме: PDF из тех же данных, что печатает страница.
//
// Данные приходят из `resume/data.json` — его пишет `just resume` из бинаря,
// поэтому даты, стаж и склонения уже посчитаны в Rust и здесь не повторяются.
// Шрифты лежат в `resume/fonts`, сборка идёт с `--font-path resume/fonts`.

#let data = json("data.json")

// Палитра и шрифты — те же, что на странице: PDF не должен выглядеть чужим.
#let ink = rgb("#16191B")
#let ink-2 = rgb("#4E585C")
#let accent = rgb("#1C4E80")
#let hair = rgb("#C6C9C2")

// Одно семейство на все начертания: узкое из Google Fonts приезжает со
// сломанным именем семейства и Typst его не видит, а разница в ширине для
// резюме не стоит отдельного файла.
#let sans = "IBM Plex Sans"

#set document(
  title: data.name + " — резюме",
  author: data.name,
  keywords: ("резюме", "backend", "Rust", "Go", "Python"),
)

#set page(
  paper: "a4",
  margin: (x: 17mm, top: 15mm, bottom: 14mm),
  footer: context {
    set text(font: sans, size: 7.5pt, fill: ink-2)
    let total = counter(page).final().first()
    grid(
      columns: (1fr, 1fr),
      align: (left, right),
      [fennr.ru],
      if total > 1 [стр. #counter(page).display() из #total],
    )
  },
)

#set text(font: sans, size: 9.2pt, fill: ink, lang: "ru")
#set par(leading: 0.62em, spacing: 0.9em)

// Адрес без схемы: в резюме читают «t.me/fenn_r», а не «https://t.me/fenn_r».
#let address(href) = {
  let text = href
    .replace("mailto:", "")
    .replace("https://", "")
    .replace("http://", "")
    .replace("www.", "")
  if text.ends-with("/") { text = text.slice(0, -1) }
  text
}

// Значение из блока «шапка»: подпись приходит из контента, поэтому в шаблоне
// нет текста, а только имена полей.
#let fact(label) = {
  let found = data.facts.find(f => f.label == label)
  if found == none { "" } else { found.value }
}

// Заголовок раздела: узкая строка капслоком и волосяная линия под ней.
#let section(title, body) = {
  v(4mm, weak: true)
  block(width: 100%)[
    #text(font: sans, size: 8.8pt, weight: 600, fill: ink-2, tracking: 0.09em)[
      #upper(title)
    ]
    #v(-1.5mm)
    #line(length: 100%, stroke: 0.5pt + hair)
    #v(-0.5mm)
    #body
  ]
}

// --- шапка ------------------------------------------------------------------

#block[
  #text(font: sans, size: 24pt, weight: 600, tracking: -0.01em)[#data.name]
  #v(-1mm)
  #text(size: 11pt, fill: ink-2)[#data.role]
  #v(0.6mm)
  #text(size: 8.8pt, fill: ink-2)[#fact("Место") · #fact("Опыт") в бэкенде]

  #v(2mm)
  #block(width: 130mm)[#data.summary]

  #v(2mm)
  #text(size: 8.6pt)[
    #data.contacts.map(contact => link(contact.href, address(contact.href))).join(
      [ #h(0.5em) · #h(0.5em) ],
    )
  ]
]

// --- о себе -----------------------------------------------------------------

// Абзацы уже колонки опыта: строки по 100 знаков читать тяжело.
#section("О себе")[
  #block(width: 135mm)[
    #for (index, paragraph) in data.about.enumerate() {
      if index > 0 { v(2.2mm, weak: true) }
      par(justify: false)[#paragraph]
    }
  ]
]

// --- опыт -------------------------------------------------------------------

#section("Опыт")[
  #for (index, role) in data.roles.enumerate() {
    if index > 0 { v(2.6mm, weak: true) }
    grid(
      columns: (26mm, 1fr),
      gutter: 5mm,
      text(size: 8pt, fill: ink-2)[#role.period],
      [
        #text(weight: 600)[#role.title]
        #v(-2.2mm)
        #text(size: 8.6pt, fill: ink-2)[#role.company, #role.place]

        #if role.summary != none [
          #v(-1.6mm)
          #role.summary
        ]
        #if role.bullets.len() > 0 [
          #v(-1.2mm)
          #set list(marker: [—], indent: 0.45em, spacing: 0.55em, body-indent: 0.45em)
          #for bullet in role.bullets { list.item(bullet) }
        ]
      ],
    )
  }
]

// --- стек -------------------------------------------------------------------

#section("Стек")[
  #for (index, group) in data.stack.enumerate() {
    if index > 0 { v(1.1mm, weak: true) }
    grid(
      columns: (26mm, 1fr),
      gutter: 5mm,
      text(size: 8pt, fill: ink-2)[#group.title],
      group.items.map(item => {
        if item.emphasis { text(weight: 600)[#item.text] } else { item.text }
      }).join([, ]),
    )
  }
]

// --- проекты ----------------------------------------------------------------

#section("Проекты")[
  #grid(
    columns: (1fr, 1fr),
    column-gutter: 7mm,
    row-gutter: 2.4mm,
    ..data.projects.map(project => [
      #link(project.href, text(weight: 600)[#project.name])
      #v(-2mm)
      #text(size: 8.4pt, fill: ink-2)[#project.what]
      #v(-1.8mm)
      #text(font: sans, size: 7.8pt, weight: 500, fill: ink-2, tracking: 0.02em)[#project.tech]
    ]),
  )
]
