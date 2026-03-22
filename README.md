# nats-cli-monitor

A real-time terminal dashboard for monitoring [NATS](https://nats.io) servers.

```
┌──────────────────────────────────────────────────────────────────────┐
│ NATS Monitor │ my-server (v2.12.5) │ Uptime: 2h30m                  │
├──────────────────────┬──────────────────────┬────────────────────────┤
│ Throughput           │ Connections          │ JetStream              │
│  Messages In  1,234  │  Connections    42   │  Streams         3    │
│  Messages Out 2,345  │  Total Conns   128   │  Consumers       9    │
│  Bytes In    50.2 MB │  Subscriptions  15   │  Stored Msgs   500    │
│  Bytes Out  120.8 MB │  Slow Consumers  0   │  API Total    4,107   │
│  CPU           1.2%  │  Memory      24.0 MB │  API Errors      5    │
├──────────────────────────────────┬───────────────────────────────────┤
│ Consumers                        │ Connections (60s)                │
│ Consumer    Stream    Dlvr  Pend │ ▁▂▃▅▇█▇▅▃▂▁▂▃▅▇█▇▅▃▂           │
│ email       JOBS      12    0    ├───────────────────────────────────┤
│ health      JOBS       5    0    │ Messages/sec (60s)               │
│ prescr      JOBS       3    0    │ ▁▁▂▃▅▇▅▃▂▁▁▂▃▅▇▅▃▂             │
└──────────────────────────────────┴───────────────────────────────────┘
```

## Features

- **Server stats** — connections, messages, bytes, CPU, memory
- **JetStream overview** — streams, consumers, stored messages, API calls
- **Consumer table** — per-consumer delivered count, ack pending, pending, redeliveries
- **Live sparkline charts** — connections and message rate over the last 60 seconds
- **Color-coded alerts** — slow consumers (red), ack pending (yellow), redeliveries (red)

## Installation

### From source

Requires [Rust](https://rustup.rs) 1.85+ (edition 2024).

```bash
cargo install --path .
```

### Build from repo

```bash
git clone https://github.com/ahmedraad/nats-cli-monitor.git
cd nats-cli-monitor
cargo build --release
# Binary is at ./target/release/nats-cli-monitor
```

## Usage

```bash
# Default: connects to http://localhost:8222
nats-cli-monitor

# Custom URL
nats-cli-monitor --url http://nats.example.com:8222

# Short form (just host:port, http:// is added automatically)
nats-cli-monitor --url nats.example.com:8222

# Custom polling interval (default: 1 second)
nats-cli-monitor --interval 5

# Combine options
nats-cli-monitor -u http://192.168.1.100:8222 -i 2
```

### Options

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--url` | `-u` | `http://localhost:8222` | NATS monitoring URL |
| `--interval` | `-i` | `1` | Polling interval in seconds |
| `--help` | `-h` | | Show help |
| `--version` | `-V` | | Show version |

### Keyboard shortcuts

| Key | Action |
|-----|--------|
| `q` / `Q` | Quit |
| `Esc` | Quit |
| `Ctrl+C` | Quit |

## Prerequisites

Your NATS server must have HTTP monitoring enabled:

```bash
# Local server
nats-server -m 8222

# With JetStream (required for stream/consumer stats)
nats-server -m 8222 -js

# Docker
docker run -p 4222:4222 -p 8222:8222 nats -m 8222 -js
```

The `-m 8222` flag enables the HTTP monitoring endpoint. Without it, the monitor cannot connect.

## How it works

The monitor polls two NATS HTTP monitoring endpoints:

- **`/varz`** — server stats (connections, messages, bytes, CPU, memory)
- **`/jsz?streams=true&consumers=true`** — JetStream stats (streams, consumers, delivered/pending counts)

No NATS client connection is needed. The monitor only uses the HTTP monitoring API, so it does not count as a client connection or interfere with your application's connections.

## License

MIT
