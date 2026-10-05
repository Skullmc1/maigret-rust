# maigret-rust

A Rust port of the core of [Maigret](https://github.com/soxoj/maigret): give it a username and it checks thousands of sites for an account with that name. It runs as a command-line tool or as a local web app.

It reads Maigret's own site database, so every site Maigret knows is checked the same way here.

## How it works

- **Database:** sites and shared "engines" are loaded from Maigret's `data.json`. A site that names an engine inherits the engine's settings.
- **Checks:** each site is requested with the username filled into its URL, and the answer is judged by the site's check type: HTTP status code, marker strings in the page, or where the request was redirected to.
- **Speed:** requests run concurrently on Tokio (50 at a time by default) with a 10 second timeout per site.
- **Web app:** an Axum server with Askama templates starts a search in the background, shows a waiting page, then lists the accounts found with their tags.

## How to run it

You need [Rust](https://rustup.rs) and a copy of Maigret's database file, [`data.json`](https://github.com/soxoj/maigret/blob/main/maigret/resources/data.json). It is not included here.

Search from the command line:

```
cargo run --release -- --db path/to/data.json --username someone
```

Or start the web app and open http://127.0.0.1:3000:

```
cargo run --release -- --db path/to/data.json --web
```

| Option | Meaning | Default |
|---|---|---|
| `-u`, `--username` | Username to search for | |
| `-d`, `--db` | Path to `data.json` | `../maigret-main/maigret/resources/data.json` |
| `-c`, `--concurrency` | Sites checked at the same time | 50 |
| `-w`, `--web` | Start the web app instead of a single search | |
| `-p`, `--port` | Web app port | 3000 |

## Not ported yet

- Extracting further IDs and linked accounts from found profiles (Maigret's `socid-extractor`), and the recursive search built on it.
- Report files (CSV, JSON, PDF, HTML) and the combined graph.
- The web form's tag and filter options; only the usernames and concurrency are used.
- Site activation, POST requests and custom probe URLs from the database.

## Credits and use

The site database and the idea belong to [Maigret](https://github.com/soxoj/maigret) by soxoj. This is an independent port and is not affiliated with it.

It only looks at public pages. Use it on your own usernames or where you have permission, and follow the law and each site's terms.
