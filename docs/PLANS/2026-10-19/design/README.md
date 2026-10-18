# Визуальные материалы Listening Atlas

Открыть `vela-atlas-board.html` в обычном браузере. Интернет не нужен: шрифты и
оригинальные геометрические иллюстрации локальны. Переключатели темы/композиции
управляют только бордом; музыка/навигация иллюстративны и отключены.

`tokens.json` — предложенные семантические значения, не внедрённый application theme.
`assets/SOURCES.json` фиксирует точные source URLs, дату, размеры и SHA256 fonts/licenses;
OFL-файлы сохраняются рядом. Эти assets не подключались к рабочему web/native client.

Скриншоты `atlas-desktop-light.png`, `atlas-desktop-dark.png`,
`atlas-phone-light.png`, `atlas-phone-dark.png`, `atlas-album-light.png` — визуальное
направление, не доказательство реализации screens/player/UX. Re-render script
`render-board.mjs` использует изолированный Playwright из existing tests/browser.
Отчёт `render-evidence.json` содержит размеры, overflow/fonts/errors и contrast pairs.

Все условные названия и время — дизайнерские примеры, не данные аккаунта. Рисунки
созданы в коде специально для Vela. Реальное application artwork в будущем берётся
из разрешённых пользовательских источников; fallback не выдаётся за обложку релиза.

## Предпросмотр

[Desktop, светлая тема](atlas-desktop-light.png) ·
[Desktop, тёмная тема](atlas-desktop-dark.png) ·
[Телефон, светлая](atlas-phone-light.png) ·
[Телефон, тёмная](atlas-phone-dark.png) ·
[Релиз](atlas-album-light.png) · [320px](atlas-narrow-light.png).
