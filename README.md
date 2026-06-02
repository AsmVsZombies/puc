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
