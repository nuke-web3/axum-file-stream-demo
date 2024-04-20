# SS streming file upload

A minimal streaming files example

You can run it with `cargo run --bin example-axum`, and then visit the documentation at `http://localhost:3000`.

## TODO

- [ ] state.rs to have some appstate?
- [ ] upload multiple files
- [ ] stream file(s) from GET to axum
- [ ] handle auto deletion
   - [ ] on successful download
   - [ ] on date experation (spawn a task for tokio::time)
