# MCP server（read-only，M2）

ReCraft 通过 **MCP（Model Context Protocol）** 把项目暴露给 AI 客户端（Claude Desktop、
Cursor、自研 agent 等）。当前里程碑只实现**只读工具 + 资源**；写操作（`rename`、
`set_comment`、`set_prototype`、`define_function`、`apply_type`）按设计走
**propose/apply 两段式**，在 M3 实现。

## 启动

```powershell
cargo run -p recraft-cli -- import C:\Windows\System32\notepad.exe -o notepad.rcraft
cargo run -p recraft-cli -- analyze notepad.rcraft
cargo run -p recraft-cli -- serve-mcp notepad.rcraft
```

传输：**stdio**，每条消息是一行 JSON-RPC 2.0（MCP stdio transport）。服务器只把
响应写到 stdout；日志/诊断走 stderr。

### HTTP 传输（`--http`）

DESIGN §9.1 的 HTTP 可选传输已实现：`serve-mcp <project> --http [--bind 127.0.0.1:8765]
[--token <token>]`。未传 `--token` 时自动生成并打印到 stderr。

```powershell
cargo run -p recraft-cli -- serve-mcp notepad.rcraft --http --bind 127.0.0.1:8765 --token demo
curl -X POST http://127.0.0.1:8765/mcp -H "Authorization: Bearer demo" `
     -H "Content-Type: application/json" -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}'
```

- `POST /mcp`：单个 JSON-RPC 消息，`Authorization: Bearer <token>` 必需（缺失/错误返回
  401，scheme 或长度不符同样拒绝）；token 用恒定时间逐字节比较，避免按响应时间猜
  token。通知（无 `id`）返回 202；解析失败 400；路径不存在 404；非 POST 405。
- `GET /health`：`{"status":"ok"}`，无需 token（只用于存活探测）。
- 读取上限：请求头 64 KiB、请求体 4 MiB；`Connection: close` 简化实现（v1）。
- 单连接串行处理，服务只会读取项目、不修改。
- 回归覆盖：`crates/recraft-mcp` 单测覆盖 token 拒绝/initialize/tools/call/health/404；
  `scripts/e2e.ps1` 会后台启动真实 HTTP 服务（随机端口 + token），断言 `/health`、携带
  token 的 `initialize` 返回 `serverInfo`、无 token 请求返回 401，然后强制结束进程。

客户端配置示例（Claude Desktop 风格）：

```json
{
  "mcpServers": {
    "recraft-notepad": {
      "command": "C:\\path\\to\\recraft-cli.exe",
      "args": ["serve-mcp", "C:\\work\\notepad.rcraft"]
    }
  }
}
```

## 协议

支持的方法：

| 方法 | 说明 |
|---|---|
| `initialize` | 回显客户端请求的 `protocolVersion`（支持 `2024-11-05`、`2025-06-18`；其他值回落 `2024-11-05`），返回 capabilities、`serverInfo.name = recraft` 与一段 `instructions` |
| `notifications/initialized` | 通知，无响应 |
| `ping` | `{}` |
| `tools/list` | 工具清单与 JSON Schema |
| `tools/call` | 调用工具；工具内部错误以 `isError: true` 返回，未知工具为 JSON-RPC `-32602` |
| `resources/list` | 集合资源（functions/strings/imports） |
| `resources/templates/list` | `pseudocode/{func}` 模板 |
| `resources/read` | 读取资源；未知 URI 为 `-32602` |

错误码：`-32700` 解析错误、`-32600` 无效请求、`-32601` 方法不存在、`-32602` 参数错误、
`-32603` 资源内部错误。

## 工具

地址参数接受：`0x...` 十六进制、十进制、符号名、`entry`。

| 工具 | 参数 | 返回 |
|---|---|---|
| `list_functions` | `filter?`, `limit?` | `{functions:[{name,start,end,size}],count,total,truncated}` |
| `get_disassembly` | `addr?`, `count?` | `{start,lines:[{addr,bytes,mnemonic,operands,name,comment}],count}` |
| `get_pseudocode` | `addr?`, `level?`=`l0/l1/l2`, `maxLines?`=2000, `maxBytes?`=256 KiB | `{function,name,level,lines,text,total_lines,truncated,hint?}` |
| `xrefs_to` | `addr`, `limit?` | `{addr,xrefs:[{from,to,kind}],count,total,truncated}` |
| `xrefs_from` | `addr`, `limit?` | 同上 |
| `list_strings` | `filter?`, `limit?` | `{strings:[{addr,length,encoding,text}],count,total,truncated}` |
| `list_imports` | — | `{modules:[{module,iat,functions:[{name,ordinal,hint,iat}]}]}` |
| `callgraph` | `limit?` | `{nodes,node_total,edges:[{from,to,from_name,to_name}],truncated}`（节点或边被裁剪都为 true） |
| `search_bytes` | `hex?` 或 `ascii?`, `limit?` | `{hits:[{addr}],count,truncated}`（扫描可读映射段，1 MiB 分块） |
| `function_summary` | `addr?` | 名字/范围/指令数/调用者/调用的函数/引用字符串/API 标签 |

**过滤与截断约定**：`filter` 先于 `limit` 生效——`limit` 只限制返回行数，不会截断扫描，
所以命中项在表的任何位置都能被找到；列表型工具都报告 `truncated`，并在适用时给出
`count`（实返行数）与 `total`（截断前的命中总数），`callgraph` 额外给出 `node_total`。
`get_pseudocode` 默认截断到 2 000 行 / 256 KiB（`maxLines`、`maxBytes` 可调，上限
100 000 行 / 8 MiB），被裁剪时返回 `truncated: true`、`total_lines` 和一句 `hint`。
客户端应先看 `truncated`，再决定收窄查询还是抬高上限，而不是把结果当成完整数据。

`tools/call` 的 `result.content[0].text` 是 **JSON 字符串**（工具结果为结构化数据），
便于客户端二次解析。

## 资源

- `recraft://project/{id}/functions` → `list_functions` 的全量 JSON
- `recraft://project/{id}/strings` → `list_strings`
- `recraft://project/{id}/imports` → `list_imports`
- `recraft://project/{id}/pseudocode/{func}` → L1 伪代码纯文本（`func` 可为名字/地址）；
  超过默认上限时末尾追加 `// truncated: N of M lines; raise maxLines/maxBytes` 注释，
  不会静默返回截短的文档

`{id}` 是项目文件名（不含扩展名）。

## 示例会话

```text
→ {"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
← {"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05",...}}

→ {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"function_summary","arguments":{"addr":"entry"}}}
← {"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"{\"name\":\"start\",...}"}],"isError":false}}
```

## 实现与测试

- 代码：`crates/recraft-mcp`（`McpServer::open` / `handle_message` / `serve_stdio`）。
- 测试：`cargo test -p recraft-mcp`（notepad 真实样本：initialize/tools/list/tools/call、
  resources/read、filter 先于 limit 的回归、伪代码截断上限、恒定时间 token 比较、未知方法与
  未知工具的错误路径）。
- E2E：`scripts/e2e.ps1` 第 11 步通过管道调用 `serve-mcp` 验证 initialize +
  `list_functions` + `resources/read`。

## 限制与后续

- **只读**：所有工具不修改项目；`rename`/注释/类型等写工具在 M3 以
  Proposal（证据地址 + 置信度）→ 人工确认 → Command 总线实现。
- 无资源订阅（`resources/subscribe`）与 MCP 提示词模板（prompts）；HTTP 传输与 bearer
  鉴权已实现（见上）。
- `search_bytes` 在 1 GiB 段上按需分块扫描，暂无通配符/正则；`callgraph` 基于已持久化
  的 call xref，未分析项目返回空图。
- `function_summary` 的 API 标签来自分析阶段写入的 `api_tag` 表。
