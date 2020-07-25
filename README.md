# jenkins-trace

A crate that provides a simple primitive to read from a [Jenkins] build log, specifically through the `/progressiveText` or `/progressiveHtml` APIs.

[Jenkins]: https://www.jenkins.io/

## Dependencies

### Library

- `reqwest`
- `bytes`
- `serde` & `serde_json`

### Binary `trace`
- `tokio`
- `url`
- `structopt`

## Features

- `tls` (enables TLS in the `reqwest` dependency)

## Usage

```
$ cargo build --bin trace
```

```bash
$ target/debug/trace --help
jenkins-trace 0.1.0

USAGE:
    trace [FLAGS] [OPTIONS] --build <build> --host <host> --job <job>

FLAGS:
        --help       Prints help information
    -H, --html       Use HTML output
    -V, --version    Prints version information

OPTIONS:
    -b, --build <build>    Numeric ID of the build
    -h, --host <host>      Jenkins host
    -j, --job <job>        Jenkins project name
    -u, --user <user>      Jenkins login credentials
```