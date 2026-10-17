<div align="center">
  <img alt="gray-mem0" src="assets/icon.svg" width="120" height="120" />
  <h1>gray-mem0</h1>
  <p><strong>Mem0 semantic memory — add, search, list, and delete long-term agent memories.</strong></p>
  <p>
    <a href="https://gray.alignment.id">Website</a> ·
    <a href="https://github.com/vstaln/gray-mem0">Source</a> ·
    <a href="https://github.com/vstaln/gray">gray</a>
  </p>
  <p>
    <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-1c1c20?style=flat-square&labelColor=0a0a0b" /></a>
    <a href="https://www.rust-lang.org"><img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-rust-1c1c20?style=flat-square&labelColor=0a0a0b&logo=rust&logoColor=d4a373" /></a>
    <img alt="gray plugin" src="https://img.shields.io/badge/gray-plugin-1c1c20?style=flat-square&labelColor=0a0a0b&color=7aa2f7" />
  </p>
</div>

<br/>
```bash
gray plugin install mem0
```

`gray-mem0` connects the agent to [Mem0](https://mem0.ai) semantic memory
over the v1 REST API — hosted at `api.mem0.ai` or any self-hosted server
exposing the same surface.

## Setup

```sh
mem0 setup --key <mem0-api-key> [--base-url https://mem0.internal] [--user alice]
mem0 status
```

Config: `~/.gray/mem0/config.json` (`{"api_key","base_url","user_id"}`) or
env `MEM0_API_KEY`, `MEM0_BASE_URL`, `MEM0_USER_ID`. `MEM0_BASE_URL`
defaults to `https://api.mem0.ai`; set it for a self-hosted backend.
Without a user scope, calls default to `user_id` `gray`.

## Tools

| Tool | What it does |
|---|---|
| `mem0_add` | Extract and store memories — `messages` (list of `{role,content}` or plain strings), optional `user_id`/`agent_id`/`run_id`, `metadata`, `infer`. |
| `mem0_search` | Semantic search — `query`, scope ids, `limit`. Returns ranked facts. |
| `mem0_list` | List memories in a scope — `user_id`/`agent_id`/`run_id`, `limit`, `page`. |
| `mem0_delete` | Delete one memory by `memory_id`, or a whole scope with `"all": true`. |

## Command

| Command | What it does |
|---|---|
| `/mem0 status` | Show base URL, key, and default user. |

## Build from source

```sh
cargo build --release
gray plugin check ./target/release/mem0
```

## License

MIT
