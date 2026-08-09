# ApriReader 1.3.1

This maintenance release makes book import more faithful and the reading view
more predictable. It focuses on the files people already have: no new account,
service, or background connection has been added.

## What changed

- FB2 titles, authors, genres, and declared covers are now recovered reliably
  from a wider range of real-world files, including large books and covers with
  wrapped Base64 data.
- Rescanning a library can repair metadata and missing cached covers without
  replacing anything you edited by hand.
- FB2 paragraphs keep inline formatting and character references together.
  Symbols such as `&`, em dashes, ellipses, and numeric references are rendered
  as readable text across FB2, EPUB, HTML, and DOCX.
- EPUB books with encoded spaces in chapter or cover paths now open correctly.
- Import feedback is clearer: ApriReader tells you how many books were added,
  refreshed, already present, or could not be read.
- Cover validation is stricter. A file is used as an embedded cover only when
  its contents really match a supported image format.

The import and reader changes were checked against the reported Flibusta
folder: all 46 books imported, all 46 exposed a title and author, 16 declared
covers were found, and all 46 opened in the reader.

## Что изменилось

- ApriReader надёжнее читает названия, авторов, жанры и обложки из FB2, в том
  числе из больших файлов и книг с переносами строк внутри Base64-обложки.
- Повторное сканирование исправляет пропущенные встроенные данные, но не
  перезаписывает название, автора, жанры или обложку, изменённые вручную.
- Текст с внутренним форматированием больше не распадается на несколько
  абзацев. Амперсанды, тире, многоточия и числовые XML/HTML-ссылки отображаются
  как обычные читаемые символы.
- EPUB с пробелами, закодированными в путях глав и обложек, открываются
  корректно.
- Итог импорта теперь честно разделяет добавленные, обновлённые, уже имеющиеся
  и не прочитанные файлы.

## Installation and privacy

ApriReader keeps source books unchanged. Metadata, progress, covers, indexes,
and backups stay in application-managed local storage. Online metadata and
optional cloud voices still run only after an explicit user action.

The Windows installer is currently unsigned, so Windows may show an unknown
publisher warning. Download it only from the official ApriReader release page
and compare its SHA-256 checksum with the value published there.
