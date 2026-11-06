# 收尾报告：多入口循环结构化 + 静态 CRT 签名库（2026-11-06）

本报告收尾两项工作：**（一）多入口循环（不可规约流图）的结构化**、**（二）为
`puts`/`__acrt_iob_func`/`fflush`/`strpbrk` 四个静态 CRT 函数建立可靠识别或可导入的签名库机制**。

结论：第二项**完成并闭环**；第一项**按可交付范围完成**——notepad 的 goto 回退函数减少
37%、crackme 减少 12.5%，剩余部分（三个巨型函数与 Go 的多 goto 函数）需要重写发射器区域模型，
已作为独立项目记录在 `docs/NEXT.md`（含设计、实测负结果与验收标准）。

## 一、签名库机制（完成）

新增 `recraft-decompile::signature` 与两条 CLI 命令：

```
recraft-cli signature-export <project> --out crt.json
recraft-cli signature-import <project> --from crt.json            # 或 --from <目录>
```

- **模式**：函数头 32 字节；相对跳转、RIP 相对位移、≥0x10000 的立即数（视作地址）通配，
  其余逐字节比对；
- **thunk**：只有一条 `jmp` 的函数没有可匹配字节，导出与匹配都透过它看真实实现；
- **冲突**：重名跳过、thunk 优先、已有名字的函数不动、`--dry-run` 可预演；
- **库**：`--from` 可重复或传目录（按文件名排序、同名先到先得），多个二进制的导出可组成一份库。

**实测**：crackme 导出 10 条签名（`main, printf, puts, fgets, fflush, strpbrk, strcmp,
strlen, __acrt_iob_func, start`），导入到全新项目后 9 个函数被命名，`main` 直接读作：

```c
void main_l1(void) {
    puts("==========================================");
    puts("  Simple Crackme v2.0  (C edition)");
    puts("  flag format: flag{...}");
    puts("==========================================");
    printf("Enter flag: ");
    __acrt_iob_func(1);
    fflush(rax);
    ...
}
```

**不需要任何原型**。跨二进制匹配要求同一份 CRT 构建（notepad 动态链接 CRT，0 匹配符合预期）。

## 二、多入口循环 / goto 回退（按可交付范围完成）

### 结果

| 样本 | goto 回退函数 | 结构化率 |
|---|---|---|
| notepad | 19 → **12**（−37%） | 96.2% → **97.6%**（487 → 494） |
| crackme | 40 → **35**（−12.5%） | 91.9% → **92.9%**（455 → 460） |
| corpus / corpus86 | 0 | 100% |
| Go | 263（不变） | 91.9% |

### 落地的 5 个文本层 pass + 1 个语义修复

1. **循环体落空补 `break`**（`4644c6a`）：`while (1)` 的体走到"不跳回"的出口时补 `break;`，
   修掉 6 张快照上的死循环/丢语句；
2. **共享尾块复制**（`3097738`）：`if (a) { P; L: T } else { Q; goto L; }` 把尾块复制进第二臂；
3. **共享尾块多引用**（`8373695`）：同一尾块被 ≤2 个分支共享时各复制一份（典型是两个 `switch`
   case 落到同一段收尾代码）；
4. **条件跳转的共享尾块**（`59c2b83`）：尾块以 `return`/`goto` 结尾（不会落空）时折叠成
   `if (c) { 尾块 }`；同时修正复制缩进（按基准缩进整体平移，保留嵌套）；
5. **函数尾部收尾块**（`2a8c861`）：`if (c) goto L;` 跳到 `return;` 之后的收尾块时折叠成
   `if (c) { 收尾块 }`；必须排在 `fold_dispatch_ladders` 之后，否则会改变阶梯折叠形状；
6. **嵌套回边包循环**（`62f55e2`）：回边落在 `if` 里时，把"标签到块尾"整段包进 `while (1)`、
   跳转改 `continue`（5 条安全条件 + 200 行闸门）；
7. **语义修复**（`db1ab8b`、`4e18c85`）：共享尾块复制要求"复制后落空位置一致"；条件跳转
   不得被无条件覆盖。

### 实测负结果（均记录在 `docs/NEXT.md`，勿重复）

| 尝试 | 结果 |
|---|---|
| 循环体扩展（并入 break 目标） | notepad 494→481、crackme 460→438 |
| 扩展 + 闩锁出口改选 | notepad 494→471、crackme 460→428 |
| 前向跳进块内部（节点复制） | 789 快照 0 变化（尾块都不以转移语句结尾） |
| 循环旋转 `goto L; do { A; L: B; }` | 生成嵌套重复循环体，语义破坏 |
| `fold_backward` 多闩锁泛化 | 789 快照 0 变化（不触发） |
| `MAX_FORWARD_SKIP` 64→200 | 0 变化 |
| 嵌套循环内 `break` 放行 | 只改 1 个 crackme 函数、指标不变 |

**根因**：`emit_linear`/`emit_leftovers` 的区域模型假设"循环体 = 自然循环"；体里多出块会让
`loop_of` 归属、`stop` 集合、左块落位全部变化，原本能规约的区域退化成平坦块。要让剩余三个
巨型函数（370/92/62 处 goto）结构化，必须按后支配树自顶向下重写发射器。

## 验证

- `cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings`、
  `cargo test --workspace` 全绿；单测 **222**（本轮目标内新增 6 个：签名模式、共享尾块 ×3、
  嵌套回边 ×2）；
- `recraft-cli lint`：crackme / notepad / corpus / corpus86 **0 findings**；
- `recraft-cli golden --check`：notepad 506、crackme 160、corpus 31、corpus86 32、go 60，
  共 **789 张全绿**；
- `recraft-cli metrics`：见上表；
- `cargo build -p recraft-ui -p recraft-cli`：`recraft.exe` / `recraft-cli.exe` 均为当前源码构建。

## 目标内提交

```
f7f43f7 feat: 函数签名导入导出（FLIRT 式命名跨项目复用）
4644c6a fix(decompile): 循环体落空到 loop exit 时补 break
3097738 feat(decompile): 共享尾块复制进跳向它的分支，消除一类 goto
8373695 feat(decompile): 共享尾块支持多引用，继续消除 goto
4e18c85 fix(decompile): 共享尾块不再折叠条件跳转（修语义错误）
59c2b83 feat(decompile): 条件跳转的共享尾块按 if (c) { 尾块 } 折叠
8278ee3 feat(cli): 签名导入支持多文件与目录（组成签名库）
2a8c861 feat(decompile): 函数尾部收尾块的跳转折叠成 if (c) { … }
db1ab8b fix(decompile): 共享尾块复制要求"复制后落空位置一致"
62f55e2 feat(decompile): 嵌套在 if 里的回边折叠成 while (1) + continue
```

## 后续工作

`docs/NEXT.md` §1.0 / §1.0.1 保留了：发射器区域模型重写的设计、节点复制的具体方案、
上述 7 条负结果与验收标准（notepad ≤8、crackme ≤30、Go 下降；lint 0 findings；789 golden 全绿）。
