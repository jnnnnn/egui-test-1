# devlog

## 2022-10-16

[try native gui](https://github.com/gabdube/native-windows-gui)

it works but windows only?

## 2022-10-21

[basalt](https://github.com/AustinJ235/basalt) seems pretty early days

## 2022-10-24

wow the egui web demo is spectacular. probably can't use it for work because no accessibility. So cool how it runs on the desktop and the web. 

## 2022-10-27

fully functional. have learned db with rusqlite, channels with crossbeam, async rust with tokio, web requests with reqwest, and gui with egui, error handling with `Box<dyn error::Error>`. Copilot has been invaluable, saves so much time. just give it a sentence about what you want to do and it suggests common libraries and lays out some code for you. It's never quite right but it's good enough to point you in the right direction.

would still like to try a better db interface, something like sqlx or diesel. also want to try a couple more gui libraries, something like iced.

binary size 11mb. shrink:
 - cargo-bloat shows text 6Mb, no large packages. 
 - https://github.com/johnthagen/min-sized-rust suggests `strip = true ` and `opt-level = "z" `. These save a couple mb. 9mb.
 - unused-features dropped the binary to 5.3MiB but broke it badly. testing the app and added back enough to get it working again. 7.3MiB. 
 - set back to `opt-level = 2`. 9MiB. 
 - https://kerkour.com/optimize-rust-binary-size suggets `lto = true`.  try with `codegen-units = 1` to start with. very slow build, 3 minutes (including 260 dependencies). 

## 2023-01-07

Can't figure out how to restore a DB.

Ah, here's what worked:

1. download the latest rar dump
2. extract the (mysql-formatted) dump from the rar
3. convert it to sqlite with mysql2sqlite: `mysql2sqlite dump.sql > dump2.sql`
4. restore it using `sqlite3 db.sqlite < dump2.sql`

Tables have changed. Easier to fix with sqlx. [Convert](https://github.com/launchbadge/sqlx#usage).

Can't figure out how to abort queries in sqlx. 

Revert back to rusqlite. At least that is straightforward to abort things, without having to have a huge pile of tokio complexity.



