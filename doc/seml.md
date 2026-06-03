# SEML 语法

SEML（Survival Endless Markup Language）用于描述 PvZ 生存无尽场景、波长、用炮、用垫和用卡操作。`puc` 支持：

```sh
puc seml [--compact] <pos|smash|explode|refresh|pogo> <文件>
```

`--compact` 只影响显示：`smash` / `refresh` / `pos` 省略明细，`explode` / `pogo` 只显示 50cs 倍数和首尾端点。

## 基本规则

- `#` 后为注释。
- 空格和 tab 会被折叠。
- `scene:` 必填，场地可用 `DE` / `NE` / `PE` / `FE` / `RE` / `ME`，大小写均可。
- 波次、行列使用 1 基编号。
- 时间支持表达式和变量，例如 `445+200`、`x+266`。
- `+N` 表示相对上一条同类时间延迟 `N`cs。

## 测试参数

```seml
scene:PE
protect:18 28 58 68
repeat:10000
std:false
avzTime:false
```

常用参数：

| 参数 | 说明 |
| --- | --- |
| `scene:` | 场地，必填 |
| `protect:` | 需要保护或预生成的位置；默认炮，加 `'` 表示普通植物 |
| `repeat:` | 模拟次数；省略时使用对应测试默认值 |
| `std:` | `true` 时显示标准误差 |
| `avzTime:` | 使用 AvZ 时间基准 |
| `cobDelay:` | `true` 时考虑炮引信延迟 |

各测试额外参数：

| 测试 | 参数 |
| --- | --- |
| `pos` | `types:` 测试僵尸类型；`targetPos:` 设置后改为到达时刻分布 |
| `refresh` | `require:` 必出类型；`ban:` 禁出类型；`huge:` 旗帜波；`activate:` 激活 / 分离；`dance:` dance cheat；`natural:` 自然出怪 |

僵尸类型可用中文单字或英文四字缩写，例如 `红白`、`giga garg`、`foot zomb`。

## 波长

```seml
w1 0 300
w 0 300
w1~4 0 1672
```

`w` 后可省略波数，由前后文自动推测。波长行格式为：

```text
w[波数] [用冰时机...] <记录/波长时机>
```

`w1~4` 会把本行及其后直到下一条 `w` 前的语句展开到 1 到 4 波。

## 用炮

```seml
P 318 2 9
PP 318 25 9
D +220 1 7
P3 318 2 9
```

- `P` / `B` / `D` 使用一门炮。
- `PP` / `BB` / `DD` 使用两门炮。
- 屋顶可在符号后加炮尾列，例如 `P3`。
- 参数为：`时间 行 列`。两门炮时行参数可写成 `25`。

## 用垫

```seml
C 446 1256 9
C 446+134 1256 9
C 446~601 1256 9
C +220+134 1256 9
C 446 1'2'56 9
```

`C` 参数为：`种植时间[铲除时间] 行 列`。

行后缀：

| 后缀 | 含义 |
| --- | --- |
| 无 | 普通垫材 |
| `'` | 小喷 / 阳光菇 |
| `"` | 花盆 |

智能垫材：

```seml
C_POS 446 1"2'56 9 choose:3 waves:1,2
C_NUM 446 1"2'56 9 choose:3
```

- `C_POS` 按红眼坐标选择行。
- `C_NUM` 按梯 / 丑总数选择行。
- `choose:` 省略时选择所有可选行。
- `waves:` 省略时考虑所有波。

## 用卡

```seml
A 225 2 9
J 318 2 1
N 400 3 9
G 776+266 5 9
a 500 2 9
W 500 2 9
```

符号：

| 符号 | 卡片 |
| --- | --- |
| `A` | 樱桃 |
| `J` | 辣椒 |
| `N` | 毁灭菇 |
| `a` / `W` | 窝瓜 |
| `G` | 大蒜 |

智能用卡：

```seml
A_NUM 225 25 9
J_NUM 225 1256 9
W_NUM 500 1256 9
```

`A_NUM` / `J_NUM` / `a_NUM` / `W_NUM` 会按巨人数量选择目标行。

## 变量

```seml
SET x 776
w1~3 0 1200~1500
C_NUM x+266 1"234'5' 9
SET x x+24
```

变量名不能是纯数字。表达式支持数字、变量、加减乘除和括号。

## 示例

砸率：

```seml
scene:PE
protect:18 28 58 68
repeat:10000

w 601
PP 225 25 9
C 539~619 1265 9
```

坐标分布：

```seml
scene:PE
types:红
repeat:20000

w 601 900
C 445+200 1256 9
```

刷新：

```seml
scene:PE
require:红白
huge:false
activate:true
dance:true
natural:true
repeat:1000

w 601
PP 225 25 9
```

更多可运行示例见 `tests/fixtures/*.seml`。
