# Website-test (Rust)

This project is now a Rust-powered GitHub profile website inspired by a propaganda-poster GitHub layout.

## Run locally

```bash
cargo run
```

Open `http://127.0.0.1:3000`.

## Configure GitHub account

Set a default account:

```bash
export GITHUB_USERNAME=your-github-username
cargo run
```

Or pass a username in the URL:

- `http://127.0.0.1:3000/?user=your-github-username`

The page pulls profile data and repositories from the GitHub API and shows linked GitHub stats cards.
