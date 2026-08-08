# ApriReader Privacy Policy / Политика конфиденциальности

Effective date / Дата вступления в силу: 2026-08-08

## English

ApriReader is a local-first Windows application maintained by the ApriReader
project. It has no user account, advertising, analytics, crash-reporting
service, or telemetry. The maintainer does not receive a user's books, library
database, reading history, annotations, statistics, generated speech, API keys,
or optional local profile name.

### Data stored on the device

ApriReader may store library metadata and source paths, reading and listening
progress, annotations, bookmarks, statistics, preferences, imported covers and
fonts, database backups, reader caches, and generated speech caches in its
application-local Windows storage. Source books and audio remain in their
original locations. Optional ElevenLabs, Google Cloud, and Azure Speech keys
are stored in Windows Credential Manager, separately from the library database.

This local data remains until the user removes it with available application
controls or deletes ApriReader's application data. Removing the application may
not automatically remove all application-local data or Windows credentials.

### Optional network actions

ApriReader performs no background catalog, telemetry, or advertising requests.
Network access occurs only after a user chooses a related action:

- Metadata search sends the entered title, author, or ISBN, selected language,
  IP address and ordinary HTTP request metadata to Open Library and Inventaire.
  Applying a result may download the selected cover from the same provider.
- Translation opens Google Translate or Yandex Translate in the default browser
  with the selected text after first-use disclosure and confirmation. The
  browser and selected provider then process that text under their own terms.
- Optional cloud narration sends bounded book fragments and the user's own API
  credential directly to the explicitly selected ElevenLabs, Google Cloud
  Text-to-Speech, or Azure AI Speech endpoint after provider-specific consent.
  The maintainer does not proxy or receive these requests.

These providers independently process network identifiers and submitted data
under their own privacy policies. Users should not send content to a provider
unless they have the right and permission to do so.

### Security and user choices

Provider credentials are never intentionally written to logs, presets, the
WebView preference store, or the library database. Users can delete a cloud
credential from the narration panel, clear generated speech caches, remove
local library records without deleting source books, and clear reading
statistics. No software can guarantee absolute security.

Because the maintainer does not receive local data, the maintainer normally
cannot inspect, export, correct, or delete it remotely. Requests concerning
data processed by an optional provider should be directed to that provider.

Privacy questions may be sent to `iambahadurrashidli@gmail.com`. Do not include
book files, API keys, credentials, or other secrets in a support request.

## Русский

ApriReader — локальное приложение для Windows, поддерживаемое проектом
ApriReader. В нём нет учётных записей, рекламы, аналитики, службы автоматической
отправки сбоев или телеметрии. Сопровождающий проекта не получает книги,
библиотечную базу, историю чтения, аннотации, статистику, созданную озвучку,
API-ключи или необязательное локальное имя пользователя.

### Данные на устройстве

ApriReader может хранить в локальном хранилище приложения метаданные и пути к
файлам, прогресс чтения и прослушивания, аннотации, закладки, статистику,
настройки, импортированные обложки и шрифты, резервные копии, кэши читалок и
созданной речи. Исходные книги и аудиофайлы остаются на прежних местах. Ключи
ElevenLabs, Google Cloud и Azure Speech хранятся отдельно в Диспетчере учётных
данных Windows.

Локальные данные сохраняются, пока пользователь не удалит их средствами
приложения или вручную из хранилища ApriReader. Удаление программы может не
удалить все локальные данные и записи Диспетчера учётных данных автоматически.

### Необязательные сетевые действия

ApriReader не выполняет фоновых запросов каталогов, телеметрии или рекламы.
Сеть используется только после явного действия пользователя:

- поиск метаданных передаёт введённое название, автора или ISBN, выбранный
  язык, IP-адрес и обычные данные HTTP-запроса Open Library и Inventaire;
  применение результата может загрузить выбранную обложку;
- перевод после предупреждения открывает Google Translate или Yandex Translate
  в браузере и передаёт выбранному сервису выделенный текст;
- облачная озвучка после отдельного согласия передаёт ограниченные фрагменты и
  собственный API-ключ пользователя напрямую выбранному ElevenLabs, Google
  Cloud Text-to-Speech или Azure AI Speech. Сопровождающий проекта не является
  посредником и не получает эти запросы.

Сторонние сервисы самостоятельно обрабатывают сетевые идентификаторы и
переданные данные по своим политикам. Пользователь не должен отправлять текст,
если у него нет необходимых прав и разрешений.

### Безопасность и управление

Ключи провайдеров намеренно не записываются в журналы, пресеты, WebView или
библиотечную базу. Пользователь может удалить ключ, очистить кэш озвучки,
удалить запись библиотеки без удаления исходного файла и очистить статистику.
Абсолютная безопасность программного обеспечения не гарантируется.

Поскольку локальные данные не передаются сопровождающему, он обычно не может
удалить, исправить или экспортировать их удалённо. По вопросам обработки данных
сторонним сервисом следует обращаться к этому сервису.

Вопросы о конфиденциальности: `iambahadurrashidli@gmail.com`. Не прикладывайте
к обращениям книги, API-ключи, учётные данные и другие секреты.
