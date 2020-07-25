# jenkins-trace

A crate that provides a simple primitive to read from [Jenkins] build log, specifically through its `/progressiveText` or `/progressiveHtml` API.

[Jenkins]: https://www.jenkins.io/

## Dependencies

- `reqwest`
- `tokio` (for the examples only)

## Features

- `tls` (enables TLS in the `reqwest` dependency)