//! mem0 — semantic memory via the Mem0 REST API.
//! Config: env MEM0_API_KEY (+ optional MEM0_BASE_URL, MEM0_USER_ID) or
//! ~/.gray/mem0/config.json {"api_key","base_url","user_id"}.

use std::io::{BufRead, Write};
use serde_json::{json, Value};

const NAME: &str = "mem0";
const DEFAULT_BASE: &str = "https://api.mem0.ai";

fn manifest() -> Value {
    json!({
        "name": NAME, "display_name": "Mem0",
        "version": env!("CARGO_PKG_VERSION"), "protocol": "1.1",
        "tools": [
            {"name": "mem0_add",
             "description": "Extract and store memories from messages. `messages` is a list of {role,content} objects or plain strings (treated as user).",
             "parameters": {"type": "object", "properties": {
                "messages": {"type": "array"},
                "user_id": {"type": "string", "description": "owner scope (default: configured user)"},
                "agent_id": {"type": "string"}, "run_id": {"type": "string"},
                "metadata": {"type": "object"},
                "infer": {"type": "boolean", "description": "extract facts (default true); false stores verbatim"}},
             "required": ["messages"]}},
            {"name": "mem0_search",
             "description": "Semantic search over stored memories; returns ranked facts.",
             "parameters": {"type": "object", "properties": {
                "query": {"type": "string"}, "user_id": {"type": "string"},
                "agent_id": {"type": "string"}, "run_id": {"type": "string"},
                "limit": {"type": "integer"}},
             "required": ["query"]}},
            {"name": "mem0_list",
             "description": "List stored memories for a user/agent/run scope.",
             "parameters": {"type": "object", "properties": {
                "user_id": {"type": "string"}, "agent_id": {"type": "string"},
                "run_id": {"type": "string"}, "limit": {"type": "integer"},
                "page": {"type": "integer"}}}},
            {"name": "mem0_delete",
             "description": "Delete one memory by id, or every memory in a scope when `all` is true.",
             "parameters": {"type": "object", "properties": {
                "memory_id": {"type": "string"},
                "all": {"type": "boolean"},
                "user_id": {"type": "string"}, "agent_id": {"type": "string"},
                "run_id": {"type": "string"}}}},
        ],
        "commands": ["/mem0"], "subcommands": ["mem0"],
    })
}

fn config_path() -> std::path::PathBuf {
    let home = std::env::var_os("GRAY_HOME").map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".gray")))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    home.join("mem0").join("config.json")
}
fn read_config() -> Value {
    std::fs::read_to_string(config_path()).ok()
        .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_else(|| json!({}))
}
fn cfg(k: &str, env: &str) -> Option<String> {
    std::env::var(env).ok().filter(|s| !s.is_empty())
        .or_else(|| read_config()[k].as_str().map(str::to_string))
}
fn base_url() -> String {
    cfg("base_url", "MEM0_BASE_URL").unwrap_or_else(|| DEFAULT_BASE.into())
}
fn user_id() -> String {
    cfg("user_id", "MEM0_USER_ID").unwrap_or_else(|| "gray".into())
}

fn api(method: &str, path: &str, body: Option<&Value>) -> Result<Value, String> {
    let base = base_url();
    let key = cfg("api_key", "MEM0_API_KEY");
    if key.is_none() && base == DEFAULT_BASE {
        return Err("no MEM0_API_KEY — set env or `mem0 setup --key …`".into());
    }
    let url = format!("{}{}", base.trim_end_matches('/'), path);
    let mut req = match method {
        "GET" => ureq::get(&url),
        "DELETE" => ureq::delete(&url),
        "PUT" => ureq::put(&url),
        _ => ureq::post(&url),
    }
    .timeout(std::time::Duration::from_secs(30))
    .set("Content-Type", "application/json");
    if let Some(k) = key { req = req.set("Authorization", &format!("Token {k}")); }
    let resp = match body { Some(b) => req.send_json(b), None => req.call() };
    match resp {
        Ok(r) => r.into_json().or_else(|_| Ok(json!({}))),
        Err(ureq::Error::Status(c, r)) => Err(format!("HTTP {c}: {}",
            r.into_string().unwrap_or_default().chars().take(300).collect::<String>())),
        Err(e) => Err(e.to_string()),
    }
}

fn enc(s: &str) -> String {
    s.chars().map(|c| match c {
        'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
        _ => format!("%{:02X}", c as u32),
    }).collect()
}

fn items(v: &Value) -> Vec<Value> {
    v["results"].as_array().cloned()
        .or_else(|| v.as_array().cloned()).unwrap_or_default()
}

fn fmt_mem(m: &Value) -> String {
    let id = m["id"].as_str().or_else(|| m["memory_id"].as_str()).unwrap_or("?");
    let text = m["memory"].as_str().or_else(|| m["text"].as_str())
        .or_else(|| m["data"]["memory"].as_str()).unwrap_or("(no text)");
    match m["score"].as_f64() {
        Some(s) => format!("{id} [{s:.3}] {text}"),
        None => match m["event"].as_str().or_else(|| m["data"]["event"].as_str()) {
            Some(e) => format!("{id} {e} {text}"),
            None => format!("{id} {text}"),
        },
    }
}

fn scope(args: &Value) -> Value {
    let mut s = json!({});
    for k in ["user_id", "agent_id", "run_id"] {
        if let Some(v) = args[k].as_str() { s[k] = json!(v); }
    }
    if s["user_id"].is_null() && s["agent_id"].is_null() && s["run_id"].is_null() {
        s["user_id"] = json!(user_id());
    }
    s
}

fn tool(name: &str, args: &Value) -> Value {
    let r = match name {
        "mem0_add" => match args["messages"].as_array() {
            Some(m) if !m.is_empty() => {
            let msgs = m.clone();
            let norm: Vec<Value> = msgs.iter().map(|m| match m {
                Value::String(t) => json!({"role": "user", "content": t}),
                Value::Object(_) => m.clone(),
                other => json!({"role": "user", "content": other.to_string()}),
            }).collect();
            let mut b = scope(args);
            b["messages"] = json!(norm);
            if args.get("metadata").is_some() { b["metadata"] = args["metadata"].clone(); }
            if let Some(i) = args["infer"].as_bool() { b["infer"] = json!(i); }
            api("POST", "/v1/memories/", Some(&b)).map(|v| {
                let its = items(&v);
                if its.is_empty() {
                    match v["status"].as_str() {
                        Some(st) => format!("add queued (status {st})"),
                        None => {
                            let s = serde_json::to_string(&v).unwrap_or_default();
                            format!("ok: {}", s.chars().take(400).collect::<String>())
                        }
                    }
                } else { its.iter().map(fmt_mem).collect::<Vec<_>>().join("\n") }
            })
            }
            _ => Err("messages must be a non-empty array of {role,content} or strings".into()),
        }
        "mem0_search" => {
            let q = match args["query"].as_str() {
                Some(q) if !q.is_empty() => q.to_string(),
                _ => return json!({"result": {"content": "query is required", "is_error": true}}),
            };
            let mut b = scope(args);
            b["query"] = json!(q);
            if let Some(n) = args["limit"].as_u64() { b["limit"] = json!(n); }
            api("POST", "/v1/memories/search/", Some(&b)).map(|v| {
                let its = items(&v);
                if its.is_empty() { "no memories matched".into() }
                else { its.iter().map(fmt_mem).collect::<Vec<_>>().join("\n") }
            })
        }
        "mem0_list" => {
            let s = scope(args);
            let mut q = vec![];
            for k in ["user_id", "agent_id", "run_id"] {
                if let Some(v) = s[k].as_str() { q.push(format!("{k}={}", enc(v))); }
            }
            q.push(format!("page_size={}", args["limit"].as_u64().unwrap_or(50).min(100)));
            q.push(format!("page={}", args["page"].as_u64().unwrap_or(1).max(1)));
            api("GET", &format!("/v1/memories/?{}", q.join("&")), None).map(|v| {
                let its = items(&v);
                if its.is_empty() { "no memories".into() }
                else { its.iter().map(fmt_mem).collect::<Vec<_>>().join("\n") }
            })
        }
        "mem0_delete" => {
            match args["memory_id"].as_str().or_else(|| args["id"].as_str()) {
                Some(id) => api("DELETE", &format!("/v1/memories/{}/", enc(id)), None)
                    .map(|_| format!("deleted {id}")),
                None if args["all"].as_bool() == Some(true) =>
                    api("DELETE", "/v1/memories/", Some(&scope(args)))
                        .map(|_| "deleted all memories in scope".into()),
                None => Err("need memory_id, or a scope with \"all\":true".into()),
            }
        }
        t => Err(format!("unknown tool {t}")),
    };
    match r {
        Ok(c) => json!({"result": {"content": if c.is_empty() {"(empty)".into()} else {c}}}),
        Err(e) => json!({"result": {"content": e, "is_error": true}}),
    }
}

fn cli(argv: &[String]) -> Result<String, String> {
    match argv.first().map(String::as_str) {
        Some("setup") => {
            let mut c = read_config();
            let mut it = argv[1..].iter();
            while let Some(f) = it.next() {
                match f.as_str() {
                    "--key" => c["api_key"] = json!(it.next().ok_or("--key needs value")?),
                    "--base-url" => c["base_url"] = json!(it.next().ok_or("--base-url needs url")?),
                    "--user" => c["user_id"] = json!(it.next().ok_or("--user needs id")?),
                    o => return Err(format!("unknown flag {o}")),
                }
            }
            std::fs::create_dir_all(config_path().parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::write(config_path(), serde_json::to_string_pretty(&c).unwrap())
                .map_err(|e| e.to_string())?;
            #[cfg(unix)] {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(config_path(),
                    std::fs::Permissions::from_mode(0o600));
            }
            Ok("saved".into())
        }
        Some("status") | None => Ok(format!(
            "Mem0\n  base_url: {}\n  api_key: {}\n  user_id: {}",
            base_url(),
            if cfg("api_key", "MEM0_API_KEY").is_some() { "set" } else { "MISSING" },
            cfg("user_id", "MEM0_USER_ID").unwrap_or_else(|| "gray (default)".into()))),
        Some("-h" | "--help" | "help") => Ok("usage: mem0 setup | status".into()),
        Some(o) => Err(format!("unknown verb {o}")),
    }
}

fn sidecar() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
        let method = v["method"].as_str().unwrap_or("");
        match v.get("id").cloned() {
            None if method == "plugin/shutdown" => return Ok(()),
            None => {}
            Some(id) => {
                let reply = if method == "plugin/manifest" {
                    json!({"id": id, "result": manifest()})
                } else if method == "tool/call" {
                    let mut r = tool(v["params"]["name"].as_str().unwrap_or(""), &v["params"]["args"]);
                    r["id"] = id; r
                } else if method == "command/run" {
                    let argv: Vec<String> = v["params"]["argv"].as_array()
                        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
                        .unwrap_or_default();
                    let text = cli(&argv).unwrap_or_else(|e| e);
                    json!({"id": id, "result": {"text": text}})
                } else {
                    json!({"id": id, "error": {"code": -32601,
                        "message": format!("method not found: {method}")}})
                };
                writeln!(out, "{reply}")?; out.flush()?;
            }
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<String> = match args.first().map(String::as_str) {
        Some("mem0") => args[1..].to_vec(), _ => args };
    match args.first().map(String::as_str) {
        Some("manifest") => println!("{}", manifest()),
        Some("sidecar") => { let _ = sidecar(); }
        _ => {
            if args.is_empty() {
                if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                    println!("mem0 — semantic memory · `mem0 setup --key <api-key> [--base-url https://x] [--user id]`");
                    return;
                }
                let _ = sidecar(); return;
            }
            match cli(&args) {
                Ok(t) => println!("{t}"),
                Err(e) => { eprintln!("{e}"); std::process::exit(1); }
            }
        }
    }
}
