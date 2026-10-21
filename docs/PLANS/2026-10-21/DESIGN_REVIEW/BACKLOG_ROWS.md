# Строки для backlog автономного тимлида

Все строки **PROPOSED**. Их можно переносить в живой BACKLOG.md без изменения мандата. Это предложения, не завершённые работы. В исходном дереве живой ../AUTONOMOUS_TEAM_LEAD/BACKLOG.md отсутствовал; существующие MANDATE/DESIGN_DIRECTIVE оставлены без правок.

Таблица использует один пакет = один пользовательский исход = один проверенный локальный коммит будущего тимлида:

| ID | Название | Приоритет | Пререквизиты | Ожидаемый пользовательский исход | Размер | Критерий готовности |
| --- | --- | --- | --- | --- | --- | --- |
| DR-01 | [Один словарь glass/motion](01_TOKEN_LAYER.md) | P2 | Нет | Темы и отклик следующих улучшений не расходятся | маленький | Canonical roles, light overrides, reduce durations0; unchanged current layout; GATE_WEB и token DOM evidence |
| DR-02 | [Glass player/topbar с безопасным underlap](02_GLASS_PLAYER_TOPBAR.md) | P2 | DR-01, DR-04 | Содержимое проходит под читаемыми полосами, последняя строка доступна | большой |2 shell blur layers/<=3 total, text>=4.5, один main-scroll/effective safe rect, fallback Firefox; GATE_WEB+scroll screenshots+trace |
| DR-03 | [Короткий отклик без задержки команды](03_MOTION_SYSTEM.md) | P2 | DR-01 | Смена cover/очереди/overlay понятна, управление остаётся немедленным | большой |80/120/140/180/150ms, transform/opacity only, instant dialog close, bounded cover layers/guards; reduced0; GATE_WEB+race/identity |
| DR-04 | [Компактное управление с44px targets](04_DENSITY_TYPOGRAPHY.md) | P1 | Нет | В Next/Queue/navigation проще попасть без разрастания UI | средний |19 исходных rect>=44, glyph15/17 сохранены, documented spacing/type exceptions; GATE_WEB+desktop/mobile regression |
| DR-05 | [Читаемые состояния и feedback](05_EMPTY_ERROR_STATES.md) | P1 | Нет | Home light читается, recovery/receipt не перекрываются, следующий шаг понятен | средний |Empty title/subtitle contrast>=3/4.5; feedback gap8/zero intersection на320–390; real states/Undo; GATE_WEB+actual state shots |
| DR-06 | [Музыка в первом mobile-экране](06_MOBILE_320_390.md) | P1 | Нет | На320 первый трек полностью виден, mobile controls и safe-area сохранены | средний |First row top<=390/350 на320/390;113px row целиком; bands120+70+safe once; GATE_WEB+TR09/render |
| DR-07 | [Различимое управление с клавиатуры](07_A11Y_MOTION_AUDIT.md) | P1 | Нет | Видимый focus и понятные range controls, предсказуемый выход из overlay | средний |Ring>=3:1,2px+3 offset; queue range44/full-width/labels; keyboard return/reduce; GATE_WEB; SR отдельно |
| DR-08 | [Проверенный Linux presentation parity](08_NATIVE_PARITY.md) | P2 | Словарь DR-01 | Native controls читаются и остаются управляемыми в WebKitGTK | большой |Native normal112px/range44, compact opaque fallback, separate window/input/audio evidence, tests и GATE_WEB; packaging/background отдельно |

Порядок: **DR-05 → DR-07 → DR-04 → DR-06 → DR-01 → DR-02 → DR-03 → DR-08**. После DR-02/03 повторить matrix DR-07 как часть приёмки этих пакетов. Между независимыми P1 допустима перестановка, но serious DV-01/02/12 исправляются раньше glass.

Пакеты технически пересекаются в style.css; тимлид выдаёт только один пакет на цикл и сверяет текущие строки. Не запускать параллельных редакторов общего CSS, Cargo/lock/миграций. Каждый worker получает сам пакет, AUDIT и README с gate/границами.

Связь с находками:

| Находки | Основной пакет | Примечание |
| --- | --- | --- |
| DV-01/02/11/13 | DR-05 | Подтверждённые читаемость/feedback/copy; не менять data semantics |
| DV-03 | DR-06 | Взаимодействие с existing safe viewport, а не уменьшение hitboxes |
| DV-04/05/08/09/15 | DR-04 | M histogram не равен числу проблем; проверить transport/seek intersection |
| DV-06 | DR-01/02 | Целевой glass gap имеет меньший приоритет usability |
| DV-07/14 | DR-01/03 | Существующий reduced-motion сохраняется |
| DV-10/12 | DR-07 | Подписи/range/focus; native отдельно |
| Native source parity | DR-08 | Web PNG не закрывает native acceptance |

В колонку фактического статуса живого backlog после реализации внесите commit, команды, артефакты и честные NOT_RUN. Ни этот документ, ни doc-only аудит не дают оснований для DONE.
