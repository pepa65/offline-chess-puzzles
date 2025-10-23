[![version](https://img.shields.io/crates/v/offline-chess-puzzles.svg)](https://crates.io/crates/offline-chess-puzzles)
[![build](https://github.com/pepa65/offline-chess-puzzles/actions/workflows/build.yml/badge.svg)](https://github.com/pepa65/offline-chess-puzzles/actions/workflows/build.yml)
[![dependencies](https://deps.rs/repo/github/pepa65/offline-chess-puzzles/status.svg)](https://deps.rs/repo/github/pepa65/offline-chess-puzzles)
[![docs](https://img.shields.io/badge/docs-offline--chess--puzzles-blue.svg)](https://docs.rs/crate/offline-chess-puzzles/latest)
[![license](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/pepa65/offline-chess-puzzles/blob/main/LICENSE)
[![downloads](https://img.shields.io/crates/d/offline-chess-puzzles.svg)](https://crates.io/crates/offline-chess-puzzles)

# offline-chess-puzzles 2.8.51
**View and solve puzzles from the lichess puzzle database**

<img src="https://github.com/pepa65/offline-chess-puzzles/blob/main/demo.gif">

A big thank you to lichess for creating the [puzzle database](https://database.lichess.org/#puzzles), to the project [chess-engine](https://github.com/adam-mcdaniel/chess-engine/) which I used as a starting point for the GUI here, and to the awesome [Iced GUI library](https://github.com/iced-rs/iced) project in which the interface is made.

* Cloned from https://github.com/brianch/offline-chess-puzzles: tweaked the clarity of messages and added Dutch translation.
* To build the binary on Linux with Rust cargo, these packages are required: `libasound2-dev` `libgtk-3-dev` `libsqlite3-dev`
* The binary needs the fitting version of libraries: `libasound2t64` `libssl3t64` `libssl3t64` `libsqlite3-0` `libgcc-s1` `libc6`

## Usage
* Download the app in the [Releases page](https://github.com/pepa65/offline-chess-puzzles/releases).
* The necessary file `lichess_db_puzzle.csv` (from the lichess link above), will be downloaded by the app if it is not found.
  By default it will be saved to the app's storage directory, but the location of the lichess database file can be changed in the `settings.json` file that will be created in the storage directory (see below). Alternatively, a symlink could be placed in the storage directory.
  - It is good to get the csv file directly so it's fresh, and can easily be replaced if needed.
* To play you simply search positions according to your needs, click `Search` and a puzzle will be loaded.
  - The initial search is somewhat slow (especially when searching by opening: it's a plaintext database).
  - For a promotion move, first select the piece to promote to (in the Search tab), before making the pawn move.
* The binary has all resources built in: font, translations, sound files, and pieces.
* The storage directory that will be used can be specified with the environment variable `OCP_HOME`.
  If not given, it will create and use the directory `.offline_chess_puzzles` in the user's home directory.
  - A settings file `settings.json` will be written to the storage directory.
  - If favorite positions are selected, they will be stored in the sqlite3 database `favorites.db` in this storage directory.

## Possible use cases
* Practice offline, it has filters by puzzle rating, theme and opening.
* Teach the tactical motifs to students, since it's simple to select easy puzzles from a theme (it lack arrows, but there's an analysis function)
* Setting a very small search limit might be useful for those who want to practice by repetition (you'll get the same puzzles each time, in random order). But there's no build-in functionality specific for this yet.

## Features
* All the filters we have in Lichess (except a few minor opening variations), plus rating range.
* Flip the board to solve from the opponent's perspective (to practice seeing what is being threatened against us).
* A number of piece themes and a bunch of board themes.
* Analysis board (with basic engine support).
* Hint (see which piece to move).
* Settings are remembered and loaded when you open the app again (`settings.json`).
* Navigate to the previous/next puzzles.
* Favorite puzzles can be remembered and searched from.
* Export part of the search to PDF.
* Save puzzle as a .jpg file.

## License:
The code is distributed under the MIT License. See `LICENSE` for more information.

### Assets authors / licenses:
* The piece set "cburnett" is a work of Colin M.L. Burnett and used under the CC-BY-SA 3.0 unported license
  (more info on the license.txt file in that directory).
* The "california" piece set is a work of Jerry S. licensed under CC BY-NC-SA 4.0
  (https://sites.google.com/view/jerrychess/home)
* The piece sets "Cardinal", "Dubrovny", "Gioco", "Icpieces", "Maestro", "Staunty", "Governor" and "Tatiana"
  are work of "sadsnake1", licensed under CC BY-NC-SA 4.0. And obtained from the lila (lichess) repository.
* The piece set and font "Chess Alpha" is a work of Eric Bentzen and free for personal non commercial use.
  Full info in the documents in the "font" directory.
* The original Merida chess font is a work of Armando Hernandez Marroquin and distributed as 'freeware'
  and the shaded version used here and obtained from the lichess repository is a work of Felix Kling
  ("DeepKling" here on github).
