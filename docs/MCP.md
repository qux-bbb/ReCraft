# MCP server（read-only，M2）

ReCraft 通过 **MCP（Model Context Protocol）** 把项目暴露给 AI 客户端（Claude Desktop、
Cursor、自研 agent 等）。默认只暴露只读工具 + 资源；加 `--write` 后额外暴露**写工具**
（`rename`、`set_comment`、`set_prototype`、`apply_type`、`define_function`、`batch` 和
`undo`），所有写操作走与 GUI 相同的 Command 总线并写入带来源标记的 journal，可以 undo
回退（`batch` 一整组一起回退）。写工具默认关闭，`serve-mcp` 不加 `--write` 时服务器对项目
的访问是只读的。

## 启动

```powershell
cargo run -p recraft-cli -- import C:\Windows\System32\notepad.exe -o notepad.rcraft
cargo run -p recraft-cli -- analyze notepad.rcraft
cargo run -p recraft-cli -- serve-mcp notepad.rcraft            # 只读
cargo run -p recraft-cli -- serve-mcp notepad.rcraft --write    # 只读 + 写工具
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
- **并发**：每个连接一个线程、每个请求一个 `Session`，一个慢反编译不会再挡住 `ping` 或
  下一个客户端；同时处理超过 32 个请求返回 503。
- **Origin 校验**：带 `Origin` 且不是 `127.0.0.1`/`localhost`/`[::1]` 的请求返回 403
  （MCP streamable HTTP 的 DNS-rebinding 防护）；不带 `Origin` 的本机客户端照常可用。
- **协议版本**：请求带 `MCP-Protocol-Version` 时必须是 `2024-11-05` 或 `2025-06-18`，
  否则 400。
- 读取上限：请求头 64 KiB、请求体 4 MiB；`Connection: close` 简化实现（v1）。
- 回归覆盖：`crates/recraft-mcp` 单测覆盖 token 拒绝/initialize/tools/call/health/404、
  Origin 拒绝与放行、协议版本校验、空闲连接不阻塞后续请求；
  `scripts/e2e.ps1` 会后台启动真实 HTTP 服务（随机端口 + token），断言 `/health`、携带
  token 的 `initialize` 返回 `serverInfo`、无 token 请求返回 401，然后强制结束进程。

客户端配置示例（Claude Desktop 风格，stdio）：

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

走 HTTP 的客户端填 **URL + Authorization 头**，没有 `command`/`args`（各客户端字段名略有
差异，下面是最常见的形态）：

```json
{
  "mcpServers": {
    "recraft-http": {
      "url": "http://127.0.0.1:8765/mcp",
      "headers": { "Authorization": "Bearer demo" }
    }
  }
}
```

- `url` 指向 `--bind` 地址下的 `/mcp` 端点，header 里的 token 必须与 `--token` 一致；
  `/health` 只用于探活，不要当 MCP 端点。
- Claude Desktop 的 `mcpServers` 只接受 `command`/`args`（stdio）；要接 HTTP 需要一个本地
  桥接（例如 `mcp-remote`），或者直接用上面的 stdio 配置。

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
| `list_functions` | `filter?`, `limit?`, `offset?` | `{functions:[{name,start,end,size}],count,total,offset,truncated,next_offset}` |
| `get_disassembly` | `addr?`, `count?` | `{start,lines:[{addr,bytes,mnemonic,operands,name,comment}],count}` |
| `get_pseudocode` | `addr?`, `level?`=`l0/l1/l2`（默认 **l2**）, `maxLines?`=2000, `maxBytes?`=256 KiB, `startLine?`, `includeLines?` | `{function,name,level,text,start_line,end_line,returned_lines,total_lines,truncated,next_start_line,hint?}`；`includeLines:true` 时额外给 `lines:[{addr,text}]` |
| `xrefs_to` | `addr`, `limit?`, `offset?` | `{addr,xrefs:[{from,to,kind}],count,total,offset,truncated,next_offset}` |
| `xrefs_from` | `addr`, `limit?`, `offset?` | 同上 |
| `list_strings` | `filter?`, `limit?`, `offset?` | `{strings:[{addr,length,encoding,text}],count,total,offset,truncated,next_offset}` |
| `list_imports` | — | `{modules:[{module,iat,functions:[{name,ordinal,hint,iat}]}]}` |
| `list_exports` | `filter?`, `limit?`, `offset?` | `{exports:[{name,ordinal,addr,rva,forwarder,function}],count,total,offset,truncated,next_offset}`（DLL 的入口面） |
| `get_metadata` | — | 格式/架构/image base/entry（含名字）/镜像与文件大小/subsystem/时间戳/TLS 回调/段（start,end,size,perm,kind）/`counts`（函数、导入、导出、字符串）/解析警告 |
| `callgraph` | `limit?`, `offset?` | `{nodes,node_total,edges:[{from,to,from_name,to_name}],edge_total,offset,truncated,next_offset}`（节点或边被裁剪都为 true） |
| `search_bytes` | `hex?` 或 `ascii?`, `limit?`, `offset?` | `{hits:[{addr}],count,total,offset,truncated,next_offset}`（扫描可读映射段，1 MiB 分块；`total` 是已检查的命中数，不是命中总数） |
| `function_summary` | `addr?` | 名字/范围/指令数/原型/调用者（数量 + 函数列表）/调用的函数/引用字符串/API 标签 |
| `find_api_callers` | `api`（导入名子串，如 `CreateFileW`）, `limit?` | `{functions:[{name,addr,api,category,site}],count,total,truncated}` |

读工具的实现与参数 schema 都在 `recraft-query`，`serve-mcp` 与内置 AI agent（`ai ask`、分析助手）
**共用同一份定义**——外部客户端和内置助手看到的是同一批工具，只是 agent 的行数/字节预算更小。

**工具元数据**：`tools/list` 里每个工具都带 `title`、`annotations`（读工具
`readOnlyHint: true`；写工具 `readOnlyHint: false` + `destructiveHint: true`，`undo`/`batch`
另有 `idempotentHint: false`）和 `outputSchema`（对象即可，暂不做逐字段约束）。`tools/call`
成功时同时返回 `content[0].text`（JSON 字符串，兼容老客户端）与 `structuredContent`
（同样的对象，2025-06-18 客户端直接消费，不必二次解析文本）。客户端可以据此自动放行只读
工具、对写工具保留确认。

### 写工具（`--write`）

写工具只在 `serve-mcp --write` 时注册；未开启时调用会在工具结果里返回 `isError: true`
和一句 “write tools are disabled … `serve-mcp --write`”。

| 工具 | 参数 | 说明 |
|---|---|---|
| `rename` | `addr`, `name`（`null` 清除） | 重命名函数/数据/导入；重名或保留名会被拒绝 |
| `set_comment` | `addr`, `text`（`null` 清除）, `kind?`=`regular/repeatable/anterior` | 行内/引用/前置注释 |
| `set_prototype` | `addr`, `text`（`null` 清除） | 函数 C 原型 |
| `apply_type` | `func`, `slot`（0 = 第一个参数）, `struct`（`null` 清除） | 把已定义的结构体绑定到参数槽 |
| `define_function` | `start`, `end`（不含端点） | 定义函数或调整范围 |
| `batch` | `commands: [{tool, arguments}]` | 一次应用多条写命令：要么全成功、要么全不落库，一条 `undo` 撤销整组 |
| `undo` | `seq?`, `dryRun?` | 撤销最新的命令或整组，并回报撤销了哪些记录 |

每个写工具返回 `seq`、改动前后的值、脏范围和 undo 提示；`seq` 之后可以喂给 `undo {seq}`
做「只撤销我以为的那一条」的校验：

```json
{"tool":"rename","seq":12,"addr":"0x140001008","previous":"sub_140001008","value":"main_entry",
 "dirty":["0x140001008..+1"],"undo":true,"note":"applied to the project journal; call undo to revert"}
```

`undo` 的结果会列出被撤销的记录（`seq`/`cmd`/`origin`/`ts`/`batch`，新→旧）：

```json
{"undone":true,"records":[{"seq":12,"cmd":"rename","origin":"mcp","ts":1762470000,"batch":7}],
 "dirty":["0x140001008..+1"]}
```

- `undo {dryRun:true}` 只回 `would_undo`，不改库，适合先确认再撤销。
- `undo {seq:N}` 要求 `N` 是**当前最新的已应用记录**，否则返回错误（“undo the newer changes
  first”），避免跳着撤销把状态弄乱；如果该记录属于一个 batch，整组一起撤销。
- `origin` 记录命令来源（`mcp`/`gui`/`cli`/`ai`/`user`）：journal 是所有前端共用的，
  审计与「这条到底是谁改的」都靠它。

语义与 GUI/CLI 完全一致：命令在同一个 SQLite 事务里写入并记录 inverse，`undo` 就是 GUI 的
撤销。项目文件同时被 GUI 打开时，GUI 需要重新载入才能看到新名字/注释。`batch` 内的地址在
提交前解析，所以同一组里不能引用本组刚创建的名字。

**过滤、分页与截断约定**：`filter` 先于 `offset`/`limit` 生效——`limit` 只限制返回行数，
不会截断扫描，所以命中项在表的任何位置都能被找到；列表型工具都报告 `truncated`，给出
`count`（实返行数）、`total`（命中总数）和 `next_offset`（下一页的 `offset`，无下一页时为
`null`），`callgraph` 额外给出 `node_total`/`edge_total`。**任何规模的表都能分页读完**：
不带 `offset` 的首页只是第一页，不是全部。
`get_pseudocode` 默认截断到 2 000 行 / 256 KiB（`maxLines`、`maxBytes` 可调，上限
100 000 行 / 8 MiB），被裁剪时返回 `truncated: true`、`start_line`/`end_line`/
`returned_lines`/`total_lines`、`next_start_line` 和一句 `hint`；用 `startLine` 从下一页接着读，
巨函数不必一次全取。默认只返回 `text`，`lines`（带每行地址）要 `includeLines:true` 才返回，
避免同一段文本在结果里出现两次。
客户端应先看 `truncated`，再决定收窄查询、翻页还是抬高上限，而不是把结果当成完整数据。

`tools/call` 的 `result.content[0].text` 是 **JSON 字符串**（工具结果为结构化数据），
便于客户端二次解析。

## 资源

- `recraft://project/{id}/functions` → `list_functions` 的全量 JSON
- `recraft://project/{id}/strings` → `list_strings`
- `recraft://project/{id}/imports` → `list_imports`
- `recraft://project/{id}/pseudocode/{func}` → L1 伪代码纯文本（`func` 可为名字/地址）；
  超过默认上限时末尾追加 `// truncated: lines 1-N of M; read the rest with
  get_pseudocode(startLine=N+1)` 注释，不会静默返回截短的文档

`{id}` 是项目文件名（不含扩展名）。

## 提示模板

`prompts/list` 提供四个模板，`prompts/get` 渲染成一条 user 消息（triage / CTF 会先注入一行
镜像事实：格式、架构、base、entry、函数/导入/导出/字符串数量，模型不必先查元数据）：

| 模板 | 参数 | 用途 |
|---|---|---|
| `triage_sample` | — | 恶意样本初筛：行为、网络/文件/注册表/进程/加密指标（带地址）、最可疑的 5 个函数、未验证项 |
| `explain_function` | `addr`（必填） | 解释单个函数：伪代码 + 调用者 + API/常量，并给出置信度 |
| `propose_names` | `filter?`（默认 `sub_`） | 给函数提命名建议（带证据与置信度），明确要求不要直接改名；`--write` 下先确认再用一次 `batch` 应用 |
| `ctf_outline` | — | crackme/CTF 解题大纲：从入口跟到输入校验，列出要读/要 patch 的地址 |

## 示例会话

```text
→ {"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
← {"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05",...}}

→ {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"function_summary","arguments":{"addr":"entry"}}}
← {"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"{\"name\":\"start\",...}"}],"isError":false}}
```

## 实现与测试

- 代码：`crates/recraft-mcp`（`McpServer::open` / `open_writable` / `handle_message` /
  `serve_stdio`）。
- 测试：`cargo test -p recraft-mcp`（notepad 真实样本：initialize/tools/list/tools/call、
  resources/read、filter 先于 limit 的回归、伪代码截断上限、恒定时间 token 比较、未知方法与
  未知工具的错误路径，以及 `--write` 的开关、rename 落库 + undo、comment/prototype 往返、
  只读服务器拒绝写工具）。
- E2E：`scripts/e2e.ps1` 第 11 步通过管道调用 `serve-mcp` 验证 initialize +
  `list_functions` + `resources/read`，并断言截断契约（`list_functions{limit:1}` 的
  `count/truncated/total`、`get_pseudocode{maxLines:1}` 的 `total_lines/truncated`）。

## 限制与后续

- **默认只读**：不加 `--write` 时所有工具都不修改用户数据。`--write` 打开的写工具走 Command
  总线并写 journal，可 `undo`；它取代了早期“MCP 只读、写操作只走 propose/apply”的约定，
  见 [ADR-014](adr/ADR-014-mcp-write-mode.md)（部分取代 [ADR-008](adr/ADR-008-mcp-propose-apply.md)）。
  只读模式唯一会写的是**派生**的伪代码缓存（`pseudocode` 表）：它可随时重建，任何编辑都会
  清空，失败也不影响返回结果；项目文件只读挂载时缓存自动失效为「每次重新反编译」。
- **反编译缓存**：`get_pseudocode` 第一次渲染后按指纹（函数范围/名字/原型/参数类型/结构体
  数量 + 反编译器版本）落库，第二次直接命中。实测 14 768 行的函数：冷 9.8 s → 热 16 ms；
  结果里 `cached` 字段标明是否命中。`includeLines` 请求逐行地址时不走缓存。
- 暂无资源订阅（`resources/subscribe`）；`prompts` 已实现（见上），HTTP 传输与 bearer
  鉴权也已实现。
- `search_bytes` 在 1 GiB 段上按需分块扫描，暂无通配符/正则；`callgraph` 基于已持久化
  的 call xref，未分析项目返回空图。
- `function_summary` 的 API 标签来自分析阶段写入的 `api_tag` 表。
