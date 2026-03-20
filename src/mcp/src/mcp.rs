use sentinel_core::client::SentinelClient;
use serde_json::{Value, json};

pub fn jsonrpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
        }
    })
}

fn jsonrpc_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

pub async fn handle_request(client: &SentinelClient, request: &Value) -> Value {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request
        .get("method")
        .and_then(|m| m.as_str())
        .unwrap_or("");

    match method {
        "initialize" => jsonrpc_result(id, json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "sentinel-mcp",
                "version": "0.1.0"
            }
        })),

        "notifications/initialized" | "notifications/cancelled" => return Value::Null,

        "ping" => jsonrpc_result(id, json!({})),

        "tools/list" => jsonrpc_result(id, json!({
            "tools": tools_list()
        })),

        "tools/call" => {
            let params = request.get("params").cloned().unwrap_or(json!({}));
            let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            handle_tool_call(client, &id, tool_name, &args).await
        }

        _ => jsonrpc_error(id, -32601, &format!("Method not found: {method}")),
    }
}

fn tools_list() -> Vec<Value> {
    vec![
        tool_def("query_host", "Get comprehensive status for a host (status, failed services, temperatures, disk)", json!({
            "type": "object",
            "properties": {
                "host": {"type": "string", "description": "Hostname (e.g., 'edge', 'mini')"}
            },
            "required": ["host"]
        })),
        tool_def("list_hosts", "List all configured hosts with connectivity status", json!({
            "type": "object",
            "properties": {}
        })),
        tool_def("check_fleet_health", "Check health of all hosts in the fleet", json!({
            "type": "object",
            "properties": {}
        })),
        tool_def("system_status", "Get system status for a host", json!({
            "type": "object",
            "properties": {
                "host": {"type": "string", "description": "Hostname"}
            },
            "required": ["host"]
        })),
        tool_def("host_temperatures", "Get hardware temperatures for a host", json!({
            "type": "object",
            "properties": {
                "host": {"type": "string", "description": "Hostname"}
            },
            "required": ["host"]
        })),
        tool_def("host_disk", "Get disk usage for a host", json!({
            "type": "object",
            "properties": {
                "host": {"type": "string", "description": "Hostname"}
            },
            "required": ["host"]
        })),
        tool_def("host_gpu", "Get GPU status for a host", json!({
            "type": "object",
            "properties": {
                "host": {"type": "string", "description": "Hostname"}
            },
            "required": ["host"]
        })),
        tool_def("view_logs", "View journal logs for a systemd unit on a host", json!({
            "type": "object",
            "properties": {
                "host": {"type": "string", "description": "Hostname"},
                "unit": {"type": "string", "description": "Systemd unit name"},
                "lines": {"type": "integer", "description": "Number of lines (default 50)", "default": 50}
            },
            "required": ["host", "unit"]
        })),
        tool_def("restart_service", "Restart a systemd service on a host (tier 1)", json!({
            "type": "object",
            "properties": {
                "host": {"type": "string", "description": "Hostname"},
                "unit": {"type": "string", "description": "Systemd unit name"}
            },
            "required": ["host", "unit"]
        })),
        tool_def("reboot_host", "Reboot a host (tier 2 — notification recommended)", json!({
            "type": "object",
            "properties": {
                "host": {"type": "string", "description": "Hostname"}
            },
            "required": ["host"]
        })),
    ]
}

fn tool_def(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

fn tool_result(content: Value) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&content).unwrap_or_default()
        }]
    })
}

fn tool_error(message: &str) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": message
        }],
        "isError": true
    })
}

async fn handle_tool_call(
    client: &SentinelClient,
    id: &Value,
    tool_name: &str,
    args: &Value,
) -> Value {
    let result = match tool_name {
        "query_host" => {
            let host = arg_str(args, "host");
            query_host(client, &host).await
        }
        "list_hosts" => list_hosts(client).await,
        "check_fleet_health" => check_fleet_health(client).await,
        "system_status" => {
            let host = arg_str(args, "host");
            system_status(client, &host).await
        }
        "host_temperatures" => {
            let host = arg_str(args, "host");
            host_temperatures(client, &host).await
        }
        "host_disk" => {
            let host = arg_str(args, "host");
            host_disk(client, &host).await
        }
        "host_gpu" => {
            let host = arg_str(args, "host");
            host_gpu(client, &host).await
        }
        "view_logs" => {
            let host = arg_str(args, "host");
            let unit = arg_str(args, "unit");
            let lines = args
                .get("lines")
                .and_then(|v| v.as_u64())
                .unwrap_or(50) as u32;
            view_logs(client, &host, &unit, lines).await
        }
        "restart_service" => {
            let host = arg_str(args, "host");
            let unit = arg_str(args, "unit");
            restart_service(client, &host, &unit).await
        }
        "reboot_host" => {
            let host = arg_str(args, "host");
            reboot_host(client, &host).await
        }
        _ => tool_error(&format!("Unknown tool: {tool_name}")),
    };

    jsonrpc_result(id.clone(), result)
}

fn arg_str(args: &Value, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

async fn query_host(client: &SentinelClient, host: &str) -> Value {
    let status = client.status(host).await;
    let failed = client.failed(host).await;
    let temps = client.temperatures(host).await;
    let disk = client.disk(host).await;

    let mut result = json!({});

    match status {
        Ok(s) => result["status"] = serde_json::to_value(&s.data).unwrap_or_default(),
        Err(e) => result["status_error"] = json!(e.to_string()),
    }
    match failed {
        Ok(f) => result["failed_services"] = serde_json::to_value(&f.data).unwrap_or_default(),
        Err(e) => result["failed_error"] = json!(e.to_string()),
    }
    match temps {
        Ok(t) => result["temperatures"] = serde_json::to_value(&t.data).unwrap_or_default(),
        Err(e) => result["temperatures_error"] = json!(e.to_string()),
    }
    match disk {
        Ok(d) => result["disk"] = serde_json::to_value(&d.data).unwrap_or_default(),
        Err(e) => result["disk_error"] = json!(e.to_string()),
    }

    tool_result(result)
}

async fn list_hosts(client: &SentinelClient) -> Value {
    let mut hosts = Vec::new();
    for host in client.hosts() {
        let reachable = client.health(host).await.is_ok();
        hosts.push(json!({
            "host": host,
            "reachable": reachable,
        }));
    }
    tool_result(json!(hosts))
}

async fn check_fleet_health(client: &SentinelClient) -> Value {
    let results = client.check_fleet_health().await;
    let mut fleet = json!({});
    let mut overall = "pass";

    for (host, result) in &results {
        match result {
            Ok(resp) => {
                if let Some(data) = &resp.data {
                    fleet[host] = serde_json::to_value(data).unwrap_or_default();
                    match data.status {
                        sentinel_core::types::HealthStatus::Fail => overall = "fail",
                        sentinel_core::types::HealthStatus::Warn if overall == "pass" => {
                            overall = "warn";
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                fleet[host] = json!({"status": "unreachable", "error": e.to_string()});
                overall = "fail";
            }
        }
    }

    tool_result(json!({
        "overall": overall,
        "hosts": fleet
    }))
}

async fn system_status(client: &SentinelClient, host: &str) -> Value {
    match client.status(host).await {
        Ok(resp) => tool_result(serde_json::to_value(&resp).unwrap_or_default()),
        Err(e) => tool_error(&format!("Failed to reach {host}: {e}")),
    }
}

async fn host_temperatures(client: &SentinelClient, host: &str) -> Value {
    match client.temperatures(host).await {
        Ok(resp) => tool_result(serde_json::to_value(&resp).unwrap_or_default()),
        Err(e) => tool_error(&format!("Failed to reach {host}: {e}")),
    }
}

async fn host_disk(client: &SentinelClient, host: &str) -> Value {
    match client.disk(host).await {
        Ok(resp) => tool_result(serde_json::to_value(&resp).unwrap_or_default()),
        Err(e) => tool_error(&format!("Failed to reach {host}: {e}")),
    }
}

async fn host_gpu(client: &SentinelClient, host: &str) -> Value {
    match client.gpu(host).await {
        Ok(resp) => tool_result(serde_json::to_value(&resp).unwrap_or_default()),
        Err(e) => tool_error(&format!("Failed to reach {host}: {e}")),
    }
}

async fn view_logs(client: &SentinelClient, host: &str, unit: &str, lines: u32) -> Value {
    match client.logs(host, unit, lines).await {
        Ok(resp) => tool_result(serde_json::to_value(&resp).unwrap_or_default()),
        Err(e) => tool_error(&format!("Failed to reach {host}: {e}")),
    }
}

async fn restart_service(client: &SentinelClient, host: &str, unit: &str) -> Value {
    match client.restart_service(host, unit).await {
        Ok(resp) => tool_result(serde_json::to_value(&resp).unwrap_or_default()),
        Err(e) => tool_error(&format!("Failed to reach {host}: {e}")),
    }
}

async fn reboot_host(client: &SentinelClient, host: &str) -> Value {
    match client.reboot(host).await {
        Ok(resp) => {
            let mut result = serde_json::to_value(&resp).unwrap_or_default();
            result["notification"] = json!("Tier 2 action: host reboot initiated. Send Pushover notification.");
            tool_result(result)
        }
        Err(e) => tool_error(&format!("Failed to reach {host}: {e}")),
    }
}
