//! The standard product quantities shared by fixed and drawn startup stories.

pub fn product_document() -> serde_json::Value {
    serde_json::json!({
      "profile":{"name":"standard"},
      "endpoints":[{"name":"codex","number":1,"dialect":1,"account":0,"provider":"codex",
        "address":"127.0.0.1:443","transport":"tls","server_name":"chatgpt.com",
        "connect":10000,"handshake":10000,
        "models":[{"name":"gpt-6-luna","window":272_000,"output":32000,"reasoning_item":65536,"prices":{"input":1,"cached":1,"output":1,"unit":1},"head":60000,"idle":300_000}]}],
      "environment":[{"name":"X","value":"x".repeat(833)}],
      "trace":{"path":"/tmp/unused-diagnostic-trace","capture":"everything"}
    })
}
