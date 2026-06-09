# puc — PvZ's Ultimate Calculator

非交互式 CLI。基于 [Rottenham/pvz-interception-calculator-rust](https://github.com/Rottenham/pvz-interception-calculator-rust)
重写命令入口：用 `clap` 提供 `puc intercept "<command>"` 子命令，结果以单行 `key=value`
形式输出至 stdout，警告与错误输出至 stderr。

## 使用

```sh
puc intercept "<command>"
```

`<command>` 中用分号 `;` 分隔多条指令；同次调用内分号前后的状态（场合、用冰/激活时机）保留。

例如：
```sh
puc intercept "pe; wave 1 400 800; delay 8.8"
# delay row=1,5 col=8.8 garg_rows=[1,2,5,6] intercept=212~230 eat=410 iceable=445
```

设置类指令（`de` / `pe` / `re`、`wave a b c`）成功时静默；查询类与计算类指令各输出一行。
错误从第一条失败处终止，进程退出码为 `1`。

## 指令

| 指令 | 说明 |
| --- | --- |
| `de` / `pe` / `re` | 设置场合（前院 / 后院 / 屋顶），静默 |
| `wave` | 查询当前用冰、激活时机，输出 `wave ice=[…] cob=… garg_x=[…,…]` |
| `wave 冰时机.. 激活时机` | 设置用冰、激活时机（用冰时机可为 0 个或多个） |
| `delay 炮列数 (炮尾列)` | 计算可拦区间、最早啃食、最早可冰<br>例：`delay 8.8`、`delay 3.5 4`（屋顶） |
| `delay2 …` / `delay3 …` | 同上，指定计算拦截两行或三行 |
| `delay 炮行数 炮列数 (炮尾列) > 巨人所在行 (巨人x范围) (u/i)` | 计算炮拦截特定巨人<br>例：`delay 1 8.8 > 1,2 700,800 u` |
| `doom 核行数 核列数 (> 巨人所在行 (巨人x范围) (u/i))` | 计算核武拦截特定巨人 |
| `hit (炮尾列) (延迟)` | 计算刚好全伤巨人的炮落点 |
| `nohit (炮尾列) (延迟)` | 计算刚好不伤巨人的炮落点 |
| `max 炮行数 炮列数范围 (炮尾列) > 巨人所在行 (巨人x范围) (u/i)` | 寻找无伤拦截可延迟最多的炮落点列 |
| `imp 巨人x坐标` | 计算该巨人投掷的小鬼x坐标范围 |
| `imp garg 小鬼x坐标(或 x1,x2)` | 计算投掷该坐标/区间小鬼的巨人x范围 |

## 计算器子命令

以下子命令源自「万能表」，为独立的 `clap` 子命令（非 `intercept` 的链式字符串）。
表格类结果按行对齐输出；可用 `--zombies key1,key2` 过滤僵尸（默认全部）。

### `puc coord` — 落点计算器

```sh
puc coord <时间> [--wave normal|flag] [--scene de|pe|re] [--kind cob|doom]
                 [--roof-tail 1-8] [--x 最小[,最大]] [--zombies ...]
```
给定波次与发炮时间，对每种僵尸查其落点 x 区间，并给出在所选场合中收上/收本/收下
各自的「全伤落点列区间」与是否可全伤。`--x` 直接指定僵尸 x 区间。

```sh
puc coord 685 --wave normal --scene pe
# coord time=685 wave=normal scene=pe kind=cob
#   僵尸  坐标范围  全伤  收上  收本  收下
#   gargantuar  718~775  √  8.2125~10  8.125~10  8.125~10  ...
```

`--kind doom` 计算核武落点：命中范围更广（上下各 3 行），输出列扩展为 7 个相对行
收上3…收本…收下3，够不到的行显示 `—`。核武落点列以整数格 1..9 显示（与 `puc time`
一致：格 G 的爆心 x = G×80），屋顶核武高度与炮尾无关，无需 `--roof-tail`。

```sh
puc coord 685 --scene pe --kind doom
# coord time=685 wave=normal scene=pe kind=doom
#   僵尸  坐标范围  全伤  收上3  收上2  收上1  收本  收下1  收下2  收下3
#   gargantuar  718~775  √  7.7625~10  6.7625~10  6.4~10  6.35~10  6.35~10  6.45~10  6.95~10
```

### `puc time` — 时机计算器

```sh
puc time <de|pe|re> <cob|doom> <行> <列> [--wave normal|flag] [--roof-tail 1-8] [--zombies ...]
```
落点计算器的逆运算：给定固定炮/核落点，对每个可伤行输出收取该行僵尸的发炮时间区间。

```sh
puc time pe cob 2 9
# time scene=pe kind=cob row=2 col=9 ... rows=1,2,3
#   僵尸  路1  路2  路3
#   gargantuar  225~1899  225~1918  225~1918  ...
```

### `puc extreme` — 慢速/快速计算器

```sh
puc extreme [--fast|--slow] [--type garg|ladder|jack] <行走时间>...
```
默认 `--fast`、`--type garg`。多个行走时间表示同行叠加（每段从该类型出生点 `x_at(type,0)` 推进，
坐标 = R − Σ(R − x_at(type, tᵢ))），对三种类型一致。`--slow` 取最慢实现、`--fast` 取最快实现。
`garg` 额外输出落点列：`--slow` 给全收两行/后院收三/前院收三，`--fast` 给正好不伤；`ladder`/`jack`
仅输出坐标。

```sh
puc extreme --slow 755
# extreme slow type=garg walk=755 coord=760.904 two_rows=7.9375 back_three=8.025 front_three=8.1125
puc extreme --type jack 200 200
# extreme fast type=jack walk=200,200 coord=643.136
```

### `puc ipp` — 热过渡

```sh
puc ipp <热过渡时机> [--wave-len 加速波波长] [--ice 冰时机]
```
给定热过渡时机，输出同收冰车与矿工的后院/前院（收二/收三）与屋顶各列炮的落点列区间。
另给 `--wave-len`（加速波波长）时，额外输出巨人坐标与炸虚落点，按卡等效（`1` 冰为完美预判冰，
默认 `--ice 1`）换算；省略 `--wave-len` 则跳过炸虚落点计算。

```sh
puc ipp 433 --wave-len 601
# ipp transition=433 wave_len=601 ice=1 garg_x=719.94 cob_col=7.4125 ...
```

### `puc seml` — SEML 模拟器

```sh
puc seml [--compact] [--strict] <pos|smash|explode|refresh|pogo> <文件>
```

解析 SEML 场景文件，调用内置 PvZ 模拟器，输出对应测试结果：

| 类型 | 说明 |
| --- | --- |
| `pos` | 坐标 / 到达时刻分布 |
| `smash` | 红眼砸率 |
| `explode` | 炮伤随时间变化 |
| `refresh` | 刷新意外率 |
| `pogo` | 跳跳全收范围 |

`--compact` 输出简表：`smash` / `refresh` / `pos` 省略明细，`explode` / `pogo`
只输出 50cs 倍数和首尾端点。`--strict` 时遇到未知行即报错；默认会跳过形如 `字段: 值`
的未知头部行，以便在多类型测试中复用同一 seml 文件。SEML 语法见 [doc/seml.md](doc/seml.md)。

```sh
puc seml smash tests/fixtures/smash.seml
puc seml --compact refresh tests/fixtures/refresh.seml
```

### 僵尸 key（`--zombies`）

`regular` `regular_dc_fast` `regular_dc_slow` `pole` `newspaper` `door` `football`
`dancing` `snorkel` `zomboni` `dolphin` `jack` `balloon` `digger` `pogo` `ladder`
`catapult` `gargantuar` `flag`，以及地形/状态变体 `duck` `duck_dc_fast` `duck_dc_slow`
`snorkel_ashore` `digger_reverse` `duck_flag` `dolphin_swim` `balloon_ground` `pogo_walk`。

## 输出约定

- 每个非静默指令输出**一行** stdout，格式为 `命令 key1=value1 key2=value2 …`。
- 拦截区间记为 `intercept=A~B`、无解记为 `intercept=cannot`、可达上限记为 `intercept=A+`。
  存在有伤区间时附加 `unsafe=A~B`。
- 警告（如 `cannot hit all gargs at this tick`、`hit col × 80 not integer`）输出至 stderr。
- 错误输出至 stderr 并以 `error:` 起首，进程返回 `1`。

## 构建

默认构建为中文消息（`zh` feature）。英文消息：
```sh
cargo build --release --no-default-features --features en
```

## 许可证

MIT。原作版权归 Crescendo 所有。
