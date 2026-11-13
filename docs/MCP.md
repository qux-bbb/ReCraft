# MCP server（默认只读，`--write` 开写工具）

ReCraft 通过 **MCP（Model Context Protocol）** 把项目暴露给 AI 客户端（OpenCode、
自研 agent 等）。默认只暴露只读工具 + 资源；加 `--write` 后额外暴露**写工具**
（`rename`、`set_comment`、`set_prototype`、`apply_type`、`define_function`、
`define_struct`、`batch` 和 `undo`），所有写操作走与 GUI 相同的 Command 总线并写入带来源标记的 journal，可以 undo
回退（`batch` 一整组一起回退）。写工具默认关闭，`serve-mcp` 不加 `--write` 时服务器对项目
的访问是只读的。

## 启动

推荐让客户端在运行时打开程序：服务器启动时是**空注册表**，`open_program` 负责导入或打开
任意 PE/ELF/`.rcraft`，同一个二进制再次打开会复用项目（名字、注释都在）：

```powershell
cargo run -p recraft-cli -- serve-mcp                 # 只读，空注册表（推荐）
cargo run -p recraft-cli -- serve-mcp --write         # 只读 + 写工具
cargo run -p recraft-cli -- serve-mcp notepad.rcraft  # 可选：启动时先打开一个项目
```

传输：**stdio**，每条消息是一行 JSON-RPC 2.0（MCP stdio transport）。服务器只把
响应写到 stdout；日志/诊断走 stderr。`--http` 的行为见下文，同样支持空注册表。

### 多实例（不绑定单个项目）

服务器不再绑定一个项目：它维护一份「已打开项目」注册表，每个项目是一个**实例**，用
短 id（项目文件名去扩展名，重名自动加 `-2`/`-3`，从二进制导入时用二进制名）寻址。

| 工具 | 参数 | 说明 |
|---|---|---|
| `open_program` | `path`（PE/ELF 或 `.rcraft`）, `project?`, `analyze?` | 导入二进制或打开已有项目并设为活动实例；新导入自动跑函数发现 + xref + 字符串。`project` 不给时数据库落在**用户目录**（Windows `%LOCALAPPDATA%\ReCraft\projects`，Linux `$XDG_DATA_HOME/recraft/projects`），文件名按二进制路径哈希，因此同一程序再次打开会复用原项目（名字/注释都在）。重复打开已打开的同一项目只切过去。`analyze:true` 对已存在的项目重跑分析 |
| `list_instances` | — | `{count, active, instances:[{id,active,project,format,arch,bits,image_base,entry,functions,strings,analyzed}]}` |
| `use_instance` | `id` | 把某个实例设为活动实例 |
| `save_project` | `path?`, `instance?` | 先 checkpoint（把 WAL 刷回项目文件），再在 `path` 与当前文件不同的情况下用 SQLite `VACUUM INTO` 写一份**一致快照**。目标已存在则拒绝（不覆盖用户项目）；不给 `path` 时复制到用户目录 |
| `close_instance` | `id?` | 关闭实例、释放数据库句柄，项目文件保留 |

其余所有工具（读工具、写工具、资源）都接受一个可选 `instance` 参数；**指定实例的调用会
同时把它设为活动实例**，所以「切一次、后面省略」也成立。`resources/list` 会列出全部实例
的资源（`recraft://project/<实例 id>/...`），`resources/read` 按 URI 里的 id 自动切换。

两个实例之间互不影响：`get_metadata`/`list_functions`/写工具都只看活动实例；`save_project`
只 checkpoint 当前实例，复制出来的文件可以再用 `open_program` 打开成第三个实例。

多实例在 **stdio 和 HTTP 会话**里都成立：stdio 的注册表属于进程；HTTP 的注册表属于
`Mcp-Session-Id` 对应的会话（没带头的简单客户端共用一个 default 会话）。
`--http` 可以带项目（新会话预打开它），也可以不带项目（新会话从空注册表开始）。


### HTTP 传输（`--http`）

`serve-mcp [project] --http [--bind 127.0.0.1:8765] [--token <token>]`。未传 `--token`
时自动生成并打印到 stderr。`project` 可以省略：每个新会话从空注册表开始，客户端用
`open_program` 打开任意程序；带项目时只是给新会话预打开一个项目。

```powershell
# 推荐：空注册表，客户端 open_program 任意程序
cargo run -p recraft-cli -- serve-mcp --http --bind 127.0.0.1:8765 --token demo

# 可选：新会话预打开一个项目（仍可 open_program 其他程序）
cargo run -p recraft-cli -- serve-mcp notepad.rcraft --http --bind 127.0.0.1:8765 --token demo
```

- `POST /mcp`：单个 JSON-RPC 消息，`Authorization: Bearer <token>` 必需（缺失/错误返回
  401，scheme 或长度不符同样拒绝）；token 用恒定时间逐字节比较，避免按响应时间猜
  token。通知（无 `id`）返回 202；解析失败 400；未知路径 404；除 `DELETE` 外的非 POST
  方法 405。
- **会话**：`initialize` 建立会话，响应头返回 `Mcp-Session-Id`；后续请求带上它。
  会话拥有自己的实例注册表，`open_program`/`use_instance`/`close_instance` 都可用。
  不带该头的简单客户端固定在 default 会话（`initialize` 也返回 `default`）。`DELETE /mcp`
  （带会话头）关闭会话，之后用旧 id 请求返回 404。
- `GET /health`：`{"status":"ok"}`，无需 token（只用于存活探测）。`GET /mcp` 不提供
  SSE 流，返回 405；本传输用 POST 的 JSON 响应承载结果。
- **并发**：每个连接一个线程；同一会话的请求由会话锁串行（一个慢反编译会挡住同一会话
  的后续调用），不同会话、`ping` 和 `/health` 不受影响；同时处理超过 32 个请求返回 503。
- **会话回收**：30 分钟没有请求的会话在下一次请求时丢弃；最多 16 个会话。
- **Origin 校验**：带 `Origin` 且不是 `127.0.0.1`/`localhost`/`[::1]` 的请求返回 403
  （MCP streamable HTTP 的 DNS-rebinding 防护）；不带 `Origin` 的本机客户端照常可用。
- **协议版本**：请求带 `MCP-Protocol-Version` 时必须是 `2024-11-05` 或 `2025-06-18`，
  否则 400。
- 读取上限：请求头 64 KiB、请求体 4 MiB；`Connection: close` 简化实现（v1）。
- 回归覆盖：`crates/recraft-mcp` 单测覆盖 token 拒绝/initialize/tools/call/health/404、
  Origin 拒绝与放行、协议版本校验、空闲连接不阻塞后续请求、会话隔离/`DELETE`/状态化
  `open_program`；`scripts/e2e.ps1` 会后台启动真实 HTTP 服务（随机端口 + token），断言
  `/health`、携带 token 的 `initialize` 返回 `serverInfo`、无 token 请求返回 401，然后
  强制结束进程。

客户端配置示例（OpenCode，stdio）：**配一次，之后用 `open_program` 打开任意程序**：

```powershell
opencode mcp add recraft -- C:\path\to\recraft-cli.exe serve-mcp
# 需要写工具时在命令末尾加 --write：
opencode mcp add recraft -- C:\path\to\recraft-cli.exe serve-mcp --write
```

`opencode mcp add` 默认写项目配置（`opencode.json`），加 `--global` 写全局
（`~/.config/opencode/opencode.jsonc`）。等价的配置文件
（OpenCode V2 形状，服务器放在 `mcp.servers` 下）：

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "servers": {
      "recraft": {
        "type": "local",
        "command": ["C:\\path\\to\\recraft-cli.exe", "serve-mcp"]
      }
    }
  }
}
```

之后在会话里 `open_program({"path": "C:\\Windows\\System32\\notepad.exe"})` 即可；再打开
第二个程序会并存为另一个实例，用 `list_instances`/`use_instance` 切换。若想固定一个项目，
把项目路径加在 `serve-mcp` 后面（`serve-mcp C:\\work\\notepad.rcraft`），仍可用
`open_program` 追加别的程序。

旧写法（服务器名直接挂在 `mcp` 下、`"enabled": true`、`command` 仍是数组）OpenCode 也照读，
已存在的配置不必迁移；新加的建议按上面的 `mcp.servers` 放。

走 HTTP 的客户端填 **URL + Authorization 头**，没有 `command`/`args`。服务端可以空注册表
启动，同一个 URL 就能打开任意程序，不用为每个样本重启：

```powershell
opencode mcp add recraft-http --url http://127.0.0.1:8765/mcp --header Authorization=Bearer demo
```

```json
{
  "mcp": {
    "servers": {
      "recraft-http": {
        "type": "remote",
        "url": "http://127.0.0.1:8765/mcp",
        "oauth": false,
        "headers": { "Authorization": "Bearer demo" }
      }
    }
  }
}
```

- `url` 指向 `--bind` 地址下的 `/mcp` 端点，header 里的 token 必须与 `--token` 一致；
  `/health` 只用于探活，不要当 MCP 端点。
- OpenCode 的 remote 服务器**默认走 OAuth**，只认 header token 的端点必须写 `oauth: false`，
  否则它会去找授权服务器。
- 验证：`opencode mcp list` 显示 `✓ recraft connected`。刚改完配置时它可能先报
  “No MCP servers configured”——list 连的是后台常驻服务、异步重载配置，等几秒或重开客户端
  再跑（`--standalone` 读不到项目配置，别用它排查）。

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
| `resources/subscribe` / `resources/unsubscribe` | 订阅/退订资源；stdio 下每次写操作（写工具或 `undo`）后发 `notifications/resources/updated`。HTTP 会话没有 push 通道，`initialize` 里 `resources.subscribe = false`，调用返回 `-32601` |

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
另有 `idempotentHint: false`）和**逐字段的 `outputSchema`**（每个结果对象的字段与类型、
以及哪些字段必然出现）。`tools/call` 成功时同时返回 `content[0].text`（JSON 字符串，
兼容老客户端）与 `structuredContent`（同样的对象，2025-06-18 客户端直接消费，不必二次
解析文本）。客户端可以据此自动放行只读工具、对写工具保留确认，并校验/渲染结构化结果。

### 写工具（`--write`）

写工具只在 `serve-mcp --write` 时注册；未开启时调用会在工具结果里返回 `isError: true`
和一句 “write tools are disabled … `serve-mcp --write`”。

| 工具 | 参数 | 说明 |
|---|---|---|
| `rename` | `addr`, `name`（`null` 清除）, `expected_previous?` | 重命名函数/数据/导入；重名或保留名会被拒绝 |
| `set_comment` | `addr`, `text`（`null` 清除）, `kind?`=`regular/repeatable/anterior/pseudo`, `expected_previous?` | 行内/引用/前置注释；`pseudo` 是反编译视图的注释（显示在行尾，与反汇编注释分开保存） |
| `set_prototype` | `addr`, `text`（`null` 清除）, `expected_previous?` | 函数 C 原型 |
| `apply_type` | `func`, `slot`（0 = 第一个参数）, `struct`（`null` 清除）, `expected_previous?` | 把已定义的结构体绑定到参数槽 |
| `define_function` | `start`, `end`（不含端点）, `expected_previous?` | 定义函数或调整范围；previous 是当前结束地址的十六进制 |
| `define_struct` | `name`, `size`, `fields`（`"offset:name:kind"` 或 `{offset,name,kind}`） | 定义/替换结构体布局（`apply_type` 的前一半，MCP 现在能自己建布局） |
| `batch` | `commands: [...]` | 一次应用多条命令：要么全成功、要么全不落库，一条 `undo` 撤销整组 |
| `undo` | `seq?`, `dryRun?` | 撤销最新的命令或整组，并回报撤销了哪些记录 |

**乐观并发（`expected_previous`）**：`rename`/`set_comment`/`set_prototype`/`apply_type`/
`define_function` 接受一个可选的 `expected_previous`。给了字符串就要求当前值完全等于它；
给 `null` 要求当前没有值；省略则跳过检查。比较和写入在**同一个 immediate 事务**里完成，
所以两个客户端不会都以为自己改的是旧值：后到的那个返回 `isError: true` 和
`stale write: … expected previous …, found …; re-read and retry`。`batch` 和
`define_struct` 没有单一 previous 值，不支持该参数（分别用逐条写工具/先读再定义）。

**批量脚本语法（`batch` 与 `recraft-cli batch` 共用）**：每条命令是
`{"type": "<命令>", ...字段}`，`type` 取 `rename` / `comment` / `prototype` /
`apply_type` / `define_function` / `define_struct`；地址可以是 `0x…`、十进制、`entry`、
已有符号，或同一脚本里刚创建的名字。`batch` 也接受 `{"tool": "rename", "arguments": {...}}`
这种与单条工具同形的写法：

```json
{"commands": [
  {"type": "rename", "addr": "sub_140001008", "name": "parse_args"},
  {"type": "comment", "addr": "parse_args", "text": "reads argv", "kind": "regular"},
  {"type": "define_struct", "name": "HDR", "size": 16,
   "fields": ["0:hdr:u32", "4:code:u32"]},
  {"type": "apply_type", "addr": "parse_args", "slot": 0, "struct": "HDR"}
]}
```

同一份语法在 `recraft-core::script`（`COMMAND_TYPES` / `parse_command`），CLI、MCP 和单条
写工具都走它——参数形状只有一处定义。

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

`{id}` 是实例 id：项目文件名（不含扩展名）或二进制名；实例重名时带 `-2`/`-3` 后缀。
`resources/list` 会为每个已打开实例各列一份。

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

- 代码：`crates/recraft-mcp`（`McpServer::open` / `open_writable` / `empty` /
  `handle_message` / `serve_stdio`；实例注册表与 `open_program` 等在 `instances.rs`）。
- 测试：`cargo test -p recraft-mcp`（notepad 真实样本：initialize/tools/list/tools/call、
  resources/read、filter 先于 limit 的回归、伪代码截断上限、恒定时间 token 比较、未知方法与
  未知工具的错误路径，以及 `--write` 的开关、rename 落库 + undo、comment/prototype 往返、
  `expected_previous` 冲突、只读服务器拒绝写工具；多实例部分覆盖空服务器提示、`open_program`
  导入到用户目录并复用、两个实例之间的调用路由与 `instance` 参数、`save_project` 的一致副本
  与拒绝覆盖、资源按实例前缀、`close_instance`；HTTP 部分覆盖会话隔离、`Mcp-Session-Id`、
  `DELETE` 关闭和状态化 `open_program`）。
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
- **多实例的作用域**：stdio 的注册表属于进程；HTTP 的注册表属于 `Mcp-Session-Id` 会话。
  不同 HTTP 会话各自独立（互不可见），同一会话的调用串行。多个前端要共享**同一个项目**
  时靠 SQLite WAL：GUI、MCP、CLI 可以同时打开同一个 `.rcraft`。
- **GUI 与 agent 共存**：GUI 轮询自己连接的 `PRAGMA data_version`；MCP/CLI 提交后版本变化，
  GUI 把函数/字符串/导入/反汇编/伪代码/CFG 标脏并在下一帧重载，不需要手动重开项目。
  GUI 自己的写入不改变该连接的数据版本，所以不会自触发刷新。两个客户端改同一字段时，
  先读后写的那个可以用 `expected_previous` 拒绝覆盖。
- 资源订阅只在 **stdio** 可用（`resources.subscribe = true`）：订阅后任何写操作都会推一条
  `notifications/resources/updated`。HTTP 会话没有 push 通道，不宣告该能力。
- `search_bytes` 在 1 GiB 段上按需分块扫描，暂无通配符/正则；`callgraph` 基于已持久化
  的 call xref，未分析项目返回空图。
- `function_summary` 的 API 标签来自分析阶段写入的 `api_tag` 表。
