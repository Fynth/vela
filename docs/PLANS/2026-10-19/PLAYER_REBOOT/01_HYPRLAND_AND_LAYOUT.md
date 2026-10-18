# Hyprland: исправить окно и геометрию, а не подобрать магическое правило

## 1. Обязательное воспроизведение перед CSS-правкой

1. Найти точный desktop executable, build commit/version и способ запуска.
2. Определить только у процесса Vela backend Wayland или XWayland; не выгружать весь
   desktop environment пользователя и список остальных открытых приложений.
3. Зафиксировать logical window bounds, framebuffer size, scale, WebView viewport,
   CSS device-pixel-ratio, доступную область после system decoration/menu.
4. Повторить tile, float, fullscreen, maximized, half/third-screen, очень низкий tile.
5. Переместить окно между мониторами с разным scale; проверить hit-test и pointer seek.
6. Отдельно размеры 640×480, 900×600, 1280×720, 1920×1080 и ultrawide3440×1440.
7. Снять полный window screenshot и content screenshot с известной геометрией,
   а не только полоску. Измерить боковые поля, top/bottom reserved space.
8. Повторить после logout/login, темы, перехода Library→Playlists→Queue и source load.
9. Проверить startup stylesheet injection/first paint: нет ли flash default WebView.
10. Протоколировать отдельно факт воспроизведения, гипотезу и подтверждённую причину.

## 2. Разделить три вида «полосы»

| Вид | Как отличить | Направление исправления |
|---|---|---|
| System titlebar / CSD | Находится вне document viewport | WindowBuilder/GTK/Dioxus decoration policy |
| Default menubar Window/Edit/Help | Menu занимает отдельную native высоту | Продуктовое desktop menu, убрать дублирование |
| CSS пустота/смещение | Входит в DOM rect/body/scroll area | Единый shell grid, margins, min-height и scroll ownership |

Не объявлять CSS margin причиной native decoration; не пытаться лечить неверный
DOM отрицательным top offset в window config. Не изменять конфигурацию Hyprland
пользователя как обязательное условие нормальной работы приложения.

## 3. Новая геометрия shell

Три горизонтальные области: navigation rail/library, основной canvas, optional
now-playing context. Нижний transport — отдельная строка shell, а не fixed overlay
с вручную подобранным padding на каждой странице. Верхняя product toolbar только одна.

Требования:

- У shell одна высота, равная доступному WebView viewport, с корректным min-size.
- Центр и optional панели получают shrinkable track sizing; min-width/min-height
  не должны неявно растягивать родительский grid.
- Scroll у content pane, library и queue явно определён; body не получает второй
  необъяснимый scroll. Transport не прокручивается вместе с песнями.
- Sidebar width учитывается ровно один раз в layout, не ещё раз в центрированном
  max-width блоке. Max readable text width задаётся внутри canvas, не сужает весь каталог.
- На ultrawide использовать дополнительное место для музыки/контекста и плотности,
  а не оставлять оба огромных поля и длинную форму посередине.
- На низком tile исчезает вторичный context, затем уплотняются header/metadata;
  Play/Pause/Next/seek остаются доступны. Player не отнимает половину высоты.
- На узком tile navigation меняет режим по реальной ширине, не по имени OS.
- Expandable errors/status не меняют padding остальных panes через старую CSS variable.
- Не применять global CSS transform/zoom для исправления scale: оно ломает hit-testing.
- Resize не пересоздаёт output, source, queue или аккаунт; только presentation.

Ориентиры размеров: desktop toolbar48–56logical px, transport80–96, compact transport
64–72; navigation220–320; вторичная панель280–420. Это предложенные размеры, а не
магические константы: при200%text/short viewport необходим доступный layout, не clipping.

## 4. Window decoration и native menu

1. Сначала проверить официальные доступные Dioxus/TAO/GTK APIs текущих pinned versions.
2. Для Linux предусмотреть standard native decoration и tested compact product chrome.
3. В compact chrome сохраняются drag/move, close, maximize, keyboard window controls,
   accessibility names и системный контекст. Не рисовать бездействующие три кружка.
4. Встроенные Window/Edit/Help либо превращаются в полезное настоящее menu, либо
   убираются поддерживаемым Config API. Preferences/Shortcuts/About/Quit не теряются.
5. На Hyprland tile не должно возникать двух больших title/toolbar полос с одинаковой
   функцией. В GNOME/KDE сохранить ожидаемое закрытие/move/window menu.
6. Не добавлять требование «всем запускать floating», force-zero-scaling или global
   GDK backend override. Различия backend — диагностический/compatibility профиль.
7. Не использовать window rules, которые захватывают другие приложения или меняют
   compositor пользователя. Optional workaround описывается отдельно и обратимо.
8. Не смешивать внешний WM frame с body фоновой полосой: цвета и высота проверяются
   на обеих темах/на старте/при inactive window.

## 5. DPI, Wayland и настоящая проверка

Матрица backend Wayland/XWayland × scale1/1.25/1.5/2 × tiled/floating × два монитора.
Результат каждой строки COMPILED/RUNS/FUNCTIONAL/NOT_RUN, не общий «Linux OK».
Текущее native QA принудительно использует process-local GDK_BACKEND=x11; это
не доказательство Wayland-native работы и не полный Hyprland tile regression.

Важные кейсы: pointer совпадает с кнопкой и seek thumb; popup/tooltip не уезжают
на другой монитор; dialog не шире window; dropdown не обрезан WebView edge; scale
не уменьшается дважды; шрифт не размывается после monitor move; fullscreen не теряет
Escape и не ломает restore bounds. Сохранять разумные bounds, не отрицательные
координаты монитора, которого больше нет.

## 6. Приёмка LAYOUT-01–08

- LAYOUT-01: ни одной необъяснимой широкой полосы, фактические rect/ratio приложены.
- LAYOUT-02: список начинается рядом с navigation по заданному gutter, без двойного offset.
- LAYOUT-03: последний трек/queue controls доступны и не перекрыты transport.
- LAYOUT-04:640×480 всё управляемо, без необходимости расширить окно на весь экран.
- LAYOUT-05:3440px не превращает контент в узкую колонку с пустыми боками.
- LAYOUT-06: смена tile/scale/монитора не меняет audio generation/position/queue.
- LAYOUT-07: window/title/menu клавиатурно доступны; no global compositor edits.
- LAYOUT-08: real package, а не только development window, проходит те же случаи.

Проверки без звука используют owned null sink; физический выход/медиа-клавиши
фиксируются отдельно. Никакого auto-play на обычные колонки пользователя ради QA.
