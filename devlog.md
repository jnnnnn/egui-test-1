# devlog

## 2022-10-16

[try native gui](https://github.com/gabdube/native-windows-gui)

it works but windows only?

## 2022-10-21

[basalt](https://github.com/AustinJ235/basalt) seems pretty early days

## 2022-10-24

wow the egui web demo is spectacular. probably can't use it for work because no
accessibility. So cool how it runs on the desktop and the web.

## 2022-10-27

fully functional. have learned db with rusqlite, channels with crossbeam, async
rust with tokio, web requests with reqwest, and gui with egui, error handling
with `Box<dyn error::Error>`. Copilot has been invaluable, saves so much time.
just give it a sentence about what you want to do and it suggests common
libraries and lays out some code for you. It's never quite right but it's good
enough to point you in the right direction.

would still like to try a better db interface, something like sqlx or diesel.
also want to try a couple more gui libraries, something like iced.

binary size 11mb. shrink:

- cargo-bloat shows text 6Mb, no large packages.
- https://github.com/johnthagen/min-sized-rust suggests `strip = true ` and
  `opt-level = "z" `. These save a couple mb. 9mb.
- unused-features dropped the binary to 5.3MiB but broke it badly. testing the
  app and added back enough to get it working again. 7.3MiB.
- set back to `opt-level = 2`. 9MiB.
- https://kerkour.com/optimize-rust-binary-size suggets `lto = true`. try with
  `codegen-units = 1` to start with. very slow build, 3 minutes (including 260
  dependencies).

## 2023-01-07

Can't figure out how to restore a DB.

Ah, here's what worked:

1. download the latest rar dump
2. extract the (mysql-formatted) dump from the rar
3. convert it to sqlite with mysql2sqlite: `mysql2sqlite dump.sql > dump2.sql`
4. restore it using `sqlite3 db.sqlite < dump2.sql`

Tables have changed. Easier to fix with sqlx.
[Convert](https://github.com/launchbadge/sqlx#usage).

Can't figure out how to abort queries in sqlx.

Revert back to rusqlite. At least that is straightforward to abort things,
without having to have a huge pile of tokio complexity.

## Popular English authors

| Author           | Results | Most popular titles                                                           |
| ---------------- | ------- | ----------------------------------------------------------------------------- |
| King, Stephen    | 4116    | Carrie, Cujo, Misery, Christine, Desperation, Dolores Claiborne, It, Insomnia |
| Roberts, Nora    | 3542    | The Perfect Neighbor, The Winning Hand, Enchanted, Rebellion, Blue Dahlia     |
| Asimov, Isaac    | 2975    | Second Foundation; Foundation; Foundation's Edge; I, Robot                    |
| Pratchett, Terry | 2960    | Eric, Equal Rites, Mort, Sourcery, Guards! Guards!                            |
| Christie, Agatha | 2947    | Nemesis, The Moving Finger, At Bertram's Hotel                                |
| Patterson, James | 2871    | Kiss the Girls, Along Came a Spider, Cross                                    |
| Anthony, Piers   | 2744    | And Eternity, Bearing an Hourglass, Heaven Cent                               |
| Resnick, Mike    | 2088    | Prophet, Kirinyaga, Oracle                                                    |
| Grant, Maxwell   | 2009    | Terror Island, Alibi Trail                                                    |


## 2023-01-09

Asked about cancelling async sqlx. You can't actually terminate the query in the engine but [you can ignore the result](https://stackoverflow.com/questions/75039196/how-to-cancel-a-long-running-query-when-using-rust-sqlx-tokio/75043208#75043208) when the async returns back to your code. I'll leave it as is for now.

## 2023-01-10

go straight to ipfs, without going through online index. This is a lot faster but requires the hashes from the db dump. Not sure how to show ISBN, can't see it in any of the tables.

Built a release:

```sh
cargo build --release
```

## 2023-01-26

tried to run again. weird error:

    thread 'main' panicked at 'called `Option::unwrap()` on a `None` value', 
    C:\Users\J\.cargo\registry\src\github.com-1ecc6299db9ec823\eframe-0.20.1\src\native\run.rs:399:58

I think this is because the window frame is wrong? the code at that point is 

```rust
std::num::NonZeroU32::new(width).unwrap()
```

but `winit_window: winit::window::Window` is (0,0) which is not nonzero. Not sure where the zeros are coming from but egui storage [says](https://github.com/emilk/egui/discussions/1698) native ?  Can't figure out how to reset this either. Tried putting some initializers in the startup but no help.

Ah it's a bug in eframe. It's already fixed but not released, using `.atleast(1)` instead of `.unwrap()`.

In order to use the fix, I can depend on the git repo instead of the cargo package.

Nope that doens't work either, some dependency conflict for the `thiserror` package -- egui requires `^1.0.37` ... but I'm locked to 1.0.32 by my cargo.lock. Ah, regenerate with `cargo update` and it works.

Add config setting to save books in an author subfolder or with a prefix.

## 2023-01-27

Starting to think about showing download status in the UI. This will require synchronization between threads. Cloning the `Book` everywhere makes this a little tricky -- do I:
 - refactor to stop cloning book (use Arc instead)
 - put an Arc to the download status inside the book and keep cloning the book itself (easier)
 - put most of the fields inside an inner BookShared object that is reference-counted (and thus shared between clones)

I think it makes the most sense to Arc the whole book. That makes accesses to the book fields a little more complicated but handling groups of books is much more efficient.

## 2023-01-30

working out a smaller DB for distribution with executable so that self-contained.

Additional SQL in [compress-db.sql](compress-db.sql).

Wow, *so* much faster with an index that matches the order-by clause. Compressed DB is now just above 100MB.  Distributable.

Try building for other platforms?! nah too hard.

Try to fix the weird thing with the table not reaching the bottom of the window. How does the debug thing work in the demos?

    structure of the ui when you hover with the mouse

```rust
ui.ctx().set_debug_on_hover(debug_on_hover);
```

well that's easy. seems like it is limiting itself.

ctrl-click into the table code. omg this is readable as. right, `max_scroll_height` defaults to 800px. Change it.  Oohhhhhhhhh yeah.

## 2023-01-31

Can I compress the database more by removing duplicates from the DB? 1500k -> 648k. would drop compressed size from 150MB to 50MB. Windows binary is 8MB.

Writing the dedup logic is too hard in sql.

## 2023-02-01

better compress script.

## 2023-04-21

In honour of Rust 1.69.0 on 20 April 2023, update dependencies.

