<h1 align="center">Zons VPN</h1>

<h3 align="center">Нативный клиент mihomo для Windows</h3>

<p align="center">
  Профили прокси, маршрутизация TUN и исключения приложений в одном окне.<br>
  Rust и Slint, программный рендеринг и работа в системном трее.
</p>

<p align="center">
  <a href="https://github.com/Goldingeng/Zons-VPN/releases/latest"><b>Скачать для Windows</b></a>
  &nbsp;·&nbsp;
  <a href="README.md">English</a> | <b>Русский</b>
</p>

<p align="center">
  <a href="https://github.com/Goldingeng/Zons-VPN/releases/latest"><img src="https://img.shields.io/github/v/release/Goldingeng/Zons-VPN?label=release" alt="Последний релиз"></a>
  <a href="https://github.com/Goldingeng/Zons-VPN/releases"><img src="https://img.shields.io/github/downloads/Goldingeng/Zons-VPN/total" alt="Скачивания"></a>
  <img src="https://img.shields.io/badge/platform-Windows%20x64-0078D4" alt="Windows x64">
  <img src="https://img.shields.io/badge/made%20with-Rust-orange" alt="Rust">
  <a href="https://slint.dev"><img src="https://img.shields.io/badge/UI-Slint-2379F4" alt="Интерфейс Slint"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/Goldingeng/Zons-VPN" alt="Лицензия GPL-3.0"></a>
</p>

---

Нативный VPN-клиент для ядра mihomo вместо Clash Party (Mihomo Party). Окно сделано на Slint с программным рендерером, есть иконка в трее. Electron и браузерный движок не используются.

## Скачать и запустить

1. Скачайте **`ZonsVPN.exe`** со страницы [последнего релиза](https://github.com/Goldingeng/Zons-VPN/releases/latest).
2. Положите его в отдельную папку и запустите. Если рядом нет `mihomo.exe`, приложение скачает ядро из [официальных релизов mihomo](https://github.com/MetaCubeX/mihomo/releases).
3. Добавьте ссылку на подписку в разделе **Профили** или положите собственный YAML-конфиг в `core\profiles\`, затем выберите активный профиль.
4. Включите системный прокси или TUN. Для **TUN** рядом с exe нужен `wintun.dll` с сайта [Wintun](https://www.wintun.net/) и запуск от администратора.

Нужна собственная совместимая подписка или YAML-конфигурация. Приложение управляет подключением и маршрутизацией, но не предоставляет VPN-серверы.

## Сборка

```
cargo build --release --ignore-rust-version
```

Slint 1.17 формально требует rustc 1.92, но собирается и на 1.91 с этим флагом.

Результат: `target\release\clashtray.exe`. В релизах этот файл называется `ZonsVPN.exe`.

## Текст

В `vendor/i-slint-renderer-software` лежит копия программного рендерера Slint 1.17.1 с правкой растеризации глифов: хинтинг TrueType, позиция глифа с шагом в четверть пикселя вместо отбрасывания дробной части и гамма 1.4 для покрытия. Без неё текст на тёмном фоне выглядит зернистым, а буквы стоят неровно. При обновлении Slint правку нужно перенести в новую версию (`fonts/vectorfont.rs` и `draw_glyph_run` в `lib.rs`) или убрать секцию `[patch.crates-io]` из `Cargo.toml`.

## Обновление ядра

Раз в сутки программа спрашивает GitHub о последнем релизе mihomo (кнопка «Обновить ядро» в настройках делает это сразу). Берётся сборка `windows-amd64-compatible`, она работает на любом x64-процессоре. Архив сверяется с SHA-256 из релиза, распаковывается встроенным в Windows `tar.exe`, затем проверяется заголовок exe и вывод `mihomo -v`. Новое ядро лежит в `core\next.exe` и ставится при следующем запуске ядра. Старое сохраняется в `core\prev.exe` и удаляется, когда новое ответило на API. Если новое ядро не запустилось, возвращается старое, а неудачная версия больше не скачивается автоматически.

## Порт

Если порт из настроек занят (например, его держит Clash Party на 7890), программа берёт следующий свободный и сохраняет его в `settings.ini`.

## Разделы

- **Обзор**: состояние ядра, системный прокси, TUN, режим (правила / глобально), скорость и текущий узел.
- **Прокси**: группы Selector, URLTest и Fallback, выбор узла, проверка задержки.
- **Профили**: добавление подписки по ссылке, обновление, удаление, выбор активного профиля.
- **Мимо VPN**: приложения, трафик которых идёт напрямую. Их можно ввести вручную, выбрать .exe или взять из списка «Сейчас в сети».
- **Подключения**: активные соединения, закрытие по одному или всех сразу.
- **Настройки**: порт, автозапуск, перезапуск ядра, журнал, папка программы, сведения о Slint.

Если закрыть окно, программа сворачивается в трей. Повторный запуск exe открывает уже запущенную копию.

## Как работают исключения

Перед запуском ядра профиль копируется в `core\run.yaml`. В начало секции `rules:` добавляются правила `PROCESS-NAME,<app>.exe,DIRECT`. Правила применяются сверху вниз, поэтому исключения срабатывают раньше правил подписки. Исходный профиль не меняется.

## Файлы

| Файл | Назначение |
|---|---|
| `subs.txt` | подписки: `имя url [user-agent]` |
| `settings.ini` | профиль, режим, порт, прокси, TUN, исключения |
| `core\profiles\*.yaml` | профили; можно класть и свои конфиги |
| `core\run.yaml` | конфиг, который реально получает ядро |
| `core\mihomo.log` | журнал ядра |
| `core\update.txt` | время последней проверки обновления и отвергнутая версия ядра |

## Лицензия

Код проекта распространяется по GPL-3.0, полный текст в `LICENSE`.

Ядро [mihomo](https://github.com/MetaCubeX/mihomo) — отдельная программа под GPL-3.0. В репозитории его нет, программа скачивает его из официальных релизов.

Интерфейс сделан на [Slint](https://slint.dev), который используется по GPL-3.0. Папка `vendor/i-slint-renderer-software` — изменённая копия рендерера Slint 1.17.1 (см. раздел «Текст»), её лицензии лежат в `vendor/i-slint-renderer-software/LICENSES`.

[![Made with Slint](https://raw.githubusercontent.com/slint-ui/slint/master/logo/MadeWithSlint-logo-whitebg.png)](https://slint.dev)
