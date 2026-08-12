# data/

```
data/
├── sources/     # importer 的输入:唯一的策展数据集,可直接手工编辑
├── gold/        # 人工核对的分词结果,segmenter 的回归集
├── build/       # 中间产物(gitignored),由 pipeline/importer 生成
└── dist/        # 只读发布库,由 pipeline/exporter 生成
```

数据流：`sources/morphemes.json` → `importer` → `build/normalized.sqlite` →
`exporter` → `dist/lexiroot.sqlite`。运行时只读 `dist/`。

三条路径在 `lexiroot-core` 里各有一个常量（`DATASET_PATH` / `BUILD_DB_PATH` /
`RELEASE_DB_PATH`），不在生产者和消费者两端各写一遍——写两遍的路径会悄悄漂移，读的
那一端会一直打开一个已经没人写的文件。

## build/

中间产物按**生产它的流水线阶段**命名（importer 的 `normalize()` → `normalized.sqlite`），
而不是叫 `processed.sqlite`：`build/` 底下的东西按定义全都是 processed 的，这个词
不携带信息，而且流水线加一级就没词可用了。

## dist/

叫 `dist/` 而不是 `release/`：在 Cargo workspace 里 `release` 已经是构建 profile
（`target/release/`），两者放在一起容易一眼看混。目录名是分发位置，"release 库"仍然
是这个制品的叫法。

发布库的路径是固定的 `dist/lexiroot.sqlite`，**版本号不进文件名**，而是写在库里
的 `meta` 表（`schema_version` / `data_version`，两者都是 `lexiroot-core` 里的常量）。

理由：文件名带版本时，这个常量会散落在每一个生产者和消费者里，升版要同步改动所有
地方，而旧文件还留在目录里——漏改的那一处会静默读到陈旧的库，不报错。改成路径恒定
之后，`store::load` 从 `meta` 读 `schema_version`，读不到或对不上就直接报错。

`meta` 里只写编译期常量，不写构建时间戳：exporter 承诺同样的输入产出逐字节相同的
文件，时间戳会破坏这个性质。对外分发时才在打包环节命名成 `lexiroot-0.2.0.sqlite`，
版本号只在那一处手写。

## sources/

只有一个文件：`morphemes.json`，4066 条词素，是整条流水线唯一的输入。

它曾经是三个文件——两个第三方词根词典加一份本项目原创的词干表——由 importer 在每次
构建时按优先级合并。现在合并结果直接固化成了这一个文件：**冲突在策展时解决一次，
写下来、看得见、可以推翻**，而不是每次构建靠优先级规则隐式裁决。

条目**不带来源标记**。三个源已经完全融入，没有什么需要区分了；两个第三方数据集是
MIT，其要求的全部就是保留版权声明，见根目录
[`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md)。

> 如果将来要引入 CC BY-SA 一类**传染性**授权的源，那时才需要重新引入逐条的来源或
> 授权标记——否则整个文件的授权会被污染。现在没有这种源，不为它预留字段。

格式：以词素的规范拼写为键的 JSON 对象，一行一条。

```json
"admit": { "positions": ["root"], "meanings": ["let in", "confess"], "examples": ["admission"], "variants": ["admiss"] }
```

| 字段 | 必需 | 含义 |
|---|---|---|
| `positions` | 是 | `prefix` / `root` / `suffix` 之一或多个 |
| `meanings` | 是 | 简短释义，**最核心的放最前**——顺序会原样保留，不排序 |
| `examples` | 否 | 由该词素构成的词；importer 会为它们预计算分解结果 |
| `variants` | 否 | **不规则**的表层异形——见下 |
| `note` | 否 | 策展说明，仅供人读，不进数据库 |

`meanings` 的顺序是有语义的。旧流水线在合并多次 sighting 时会去重并因此**按字母序
重排** meanings，把「最核心的放最前」这个判断洗掉；新的 parser 一条就是一个词素，
没有 sighting 需要折叠，所以顺序原样落库。

importer 对这个文件是严格的：位置拼错、`positions` 为空、两个键只有大小写不同（会
在小写化的 id 上撞车），都直接让构建失败，而不是静默丢弃——这是我们自己的文件，
出错就是笔误。

## 策展规则

数据集的希腊语 / 拉丁语部分来自两部**黏着词根**词典，擅长 `spect`、`port`、`struct`
这类不能独立成词的形式，而英语原生的日耳曼语核心几乎完全不在其中。

后果是全面的而非局部的：`believe`、`help`、`friend`、`understand`、`break`、`speak`
若不在词素表里，任何基于它们构成的词都无法分词。`unbelievable` 会失败——即便 `un-`
和 `-able` 都存在——因为中间的词根槽解析出 `believ`，匹配不到任何东西。本项目原创的
自由词干条目补的就是这个缺口，扩充数据集时优先补它。

### 什么该进 `variants`，什么不该

`variants` 只放**没有通则可预测**的交替形式，必须逐条列出：

- 拉丁语词干交替 —— `admit` ~ `admiss`、`receive` ~ `recept`
- 前缀同化 —— `in-` ~ `im-`、`il-`、`ir-`

词素边界上的常规英语拼写调整**不列在这里**：

| 变化 | 例子 | 由谁处理 |
|---|---|---|
| 去哑音 e | `believe` + `-able` → `believable` | `analyzer::ortho::SilentE` |
| 辅音双写 | `run` + `-ing` → `running` | `analyzer::ortho::Undouble` |
| `y` → `i` | `happy` + `-ness` → `happiness` | `analyzer::ortho::YToI` |

这些是能产的：它们适用于每一个词干，包括这个文件从没收录过的。把它们枚举出来意味着
每个词干多出三四行无效数据，而且照样漏掉新的情况。分词器自己推导。

所以：**规则能预测的，就别写进来。**

### 收录规则

- 以规范拼写为键 —— 用词元，不用屈折形式。
- 至少三个字符。分词器的 `MIN_ROOT_LEN` 是 3，更短的形式会把词切成噪音。
- 不把已经是能产词缀的形式再登记成词根（`ship`、`ward`、`less`、`hood`、`ment`、`dom`）。
  搜索会优先选中它们而不是真正的词根。
- 每次新增都要对着 `data/gold/segmentations.tsv` 检查 —— 破坏已有正确分词的词干不收。
