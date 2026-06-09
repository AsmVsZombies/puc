# PvZ's Ultimate Calculator

集成诸常用键控炮阵计算器的非交互式 CLI / MCP 工具，便于 AI Agent 使用。

采用的时间单位为 cs，取刷新等效时间基准（`seml` 子命令仍以 `avzTime` 字段为准）；列单位为格（1 格 = 80 px），以炮落点为基准。

## 构建与使用

```sh
# 构建
cargo build --release

# 添加至 Claude Code
claude mcp add --scope user --transport stdio puc "C:\path\to\puc" mcp-server

# 添加至 Codex
codex mcp add puc "C:\path\to\puc" mcp-server
```

输出语言在运行时选择（`--lang zh|en` 或 `PUC_LANG` 环境变量，默认 `zh`）。

不需要 MCP 服务端时可去掉异步栈：`cargo build --release --no-default-features`。

## 子命令

### `puc intercept "<command>"`: 拦截计算器

有状态指令流：同次调用内用 `;` 分隔多条指令，场合与波次状态在分号间保留。

```sh
puc intercept "pe; wave 1 400 800; delay 8.8"
# delay row=1,5 col=8.8 garg_rows=[1,2,5,6] intercept=212~230 eat=410 iceable=445
```

完整语法：`puc://docs/intercept`（MCP 资源）或 [doc/intercept.md](doc/intercept.md)。

### `puc coord`: 落点计算器

给定发炮时间，输出各僵尸的落点 x 区间与全伤落点列区间。

```sh
puc coord <时间> [--wave normal|flag] [--scene de|pe|re] [--kind cob|doom]
                 [--roof-tail 1-8] [--x 最小[,最大]] [--zombies ...]
```

`--kind doom` 计算核武落点：命中范围上下各 3 行，输出 7 列（收上3…收本…收下3）。

### `puc time`: 时机计算器

落点计算器的逆运算：给定落点，输出收取各行僵尸的发炮时间区间。

```sh
puc time <de|pe|re> <cob|doom> <行> <列> [--wave normal|flag] [--roof-tail 1-8] [--zombies ...]
```

### `puc extreme`: 快慢速计算器

计算快速/慢速巨人（或跳跳、顶车）在给定行走时间后的坐标，并给出对应全收落点列。

```sh
puc extreme [--fast|--slow] [--type garg|ladder|jack] <行走时间>...
```

多个行走时间为同行叠加段。`garg` 额外输出落点列；`ladder`/`jack` 仅输出坐标。

### `puc ipp`: 热过渡

给定热过渡时机，输出各区域炮的同收落点列区间。  
加 `--wave-len` 时额外输出巨人坐标与炸虚落点。

```sh
puc ipp <热过渡时机> [--wave-len 加速波波长] [--ice 冰时机]
```

### `puc seml`: SEML 模拟器

解析 SEML 场景文件，调用内置 PvZ 模拟器，输出模拟测试结果。

```sh
puc seml [--compact] [--strict] [--csv <目标>] <类型> <文件>
```

| 类型 | 说明 |
| --- | --- |
| `pos` | 坐标 / 到达时刻分布 |
| `smash` | 红眼砸率 |
| `explode` | 炮伤随时间变化 |
| `refresh` | 刷新意外率 |
| `pogo` | 跳跳全收范围 |
| `reuse` | 用炮复用计算（纯时间计算，不跑模拟器） |

`--compact` 省略明细；`--strict` 遇未知行报错（默认跳过）；`--csv` 额外导出 CSV。

完整语法：`puc://docs/seml`（MCP 资源）或 [doc/seml.md](doc/seml.md)。

### `puc mcp-server`: MCP 服务端

将所有子命令作为工具暴露，通过标准 stdio 提供 MCP 服务。

## 致谢

- [拦截计算器](https://github.com/Rottenham/pvz-interception-calculator-rust)（`puc intercept` 基于此重写）
- [万能表](https://www.bilibili.com/opus/952670329504792593)（`coord` / `time` / `extreme` / `ipp` 数据来源）
- [pvz-emulator](https://github.com/Rottenham/pvz-emulator-examples)（`puc seml` 内置模拟器）
- [SEML](https://github.com/Rottenham/seml)（SEML 语法）
