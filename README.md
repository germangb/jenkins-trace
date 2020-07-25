# jenkins-trace

A crate that provides a simple primitive to read from a [Jenkins] build log, specifically through the `/progressiveText` or `/progressiveHtml` APIs.

[Jenkins]: https://www.jenkins.io/

## Dependencies

- `reqwest`
- `bytes`
- `serde` & `serde_json`

## Features

- `tls` (enables TLS in the `reqwest` dependency)