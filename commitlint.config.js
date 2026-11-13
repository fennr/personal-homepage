// Сообщения коммитов: Conventional Commits без трейлера Co-authored-by.
//
// Версия проекта живёт в Cargo.toml, поэтому здесь её нет: одна версия,
// которую сверяет релизный шаг.
export default {
  extends: ["@commitlint/config-conventional"],
  plugins: [
    {
      rules: {
        "no-co-authored-by": (parsed) => {
          const message = [parsed.body, parsed.footer]
            .filter(Boolean)
            .join("\n");
          return [
            !/^Co-authored-by:/imu.test(message),
            "Co-authored-by trailers are prohibited",
          ];
        },
      },
    },
  ],
  rules: {
    "no-co-authored-by": [2, "always"],
  },
};
