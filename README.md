# jenkins-trace

A crate that provides a simple primitive to read from [Jenkins] build log, specifically through its `/progressiveText` or `/progressiveHtml` API.

[Jenkins]: https://www.jenkins.io/

## Dependencies

- `reqwest`
- `bytes`
- `serde` & `serde_json`

## Features

- `tls` (enables TLS in the `reqwest` dependency)