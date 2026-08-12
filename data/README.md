# data/

```
data/
├── sources/     # importer 的输入,一个源一个文件,可直接手工编辑
├── gold/        # 人工核对的分词结果,segmenter 的回归集
├── build/       # 中间产物(gitignored),由 pipeline/importer 生成
└── release/     # 只读发布库,由 pipeline/exporter 生成
```

数据流：`sources/*.json` → `importer` → `build/processed.sqlite` → `exporter` →
`release/lexiroot-vX.Y.sqlite`。运行时只读 `release/`。

## sources/

三个文件都是 LexiRoot 自己维护的数据集，其中两个以第三方数据为起点、之后由本项目
自行修改和扩充。它们**不是**上游仓库的镜像，不跟随上游更新，可以直接编辑。

每个文件对应 `SourceId` 的一个取值，这个标识会写进数据库的 `source` 列：importer
在合并阶段依赖它判断一条词根是否被多个源同时收录（多源一致 → 置信度更高），所以
三个文件保持独立，不合并成一个。

| 文件 | `SourceId` | 内容 | 起点 |
|---|---|---|---|
| `colingoldberg-morphemes.json` | `colingoldberg_morphemes` | 2435 条前缀 / 词根 / 后缀，带词义和例词 | [colingoldberg/morphemes](https://github.com/colingoldberg/morphemes)（MIT）的 `data/morphemes.json` |
| `withenglishwecan-roots.json` | `withenglishwecan_roots` | 1061 条希腊语 / 拉丁语词根 | [WithEnglishWeCan/generated-english-roots-list](https://github.com/WithEnglishWeCan/generated-english-roots-list)（MIT）的 `english.roots.list.build.json` |
| `lexiroot-stems.json` | `lexiroot_stems` | 英语原生自由词干 + 不规则异形 | 本项目原创 |

许可证声明见根目录 [`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md)。

## lexiroot-stems.json 的策展规则

上面两个第三方数据集都是希腊语 / 拉丁语的**黏着词根**词典，擅长 `spect`、`port`、
`struct` 这类不能独立成词的形式，而英语原生的日耳曼语核心几乎完全不在其中。

后果是全面的而非局部的：`believe`、`help`、`friend`、`understand`、`break`、`speak`
都不在词素表里，任何基于它们构成的词都无法分词。`unbelievable` 会失败——即便 `un-`
和 `-able` 都存在——因为中间的词根槽解析出 `believ`，匹配不到任何东西。这个文件补上
这些词干。

格式：以词干的规范拼写为键的 JSON 对象。

```json
"believe": {
  "positions": ["root"],
  "meanings": ["accept as true", "have faith in"],
  "examples": ["believable", "unbelievable", "believer"]
}
```

| 字段 | 必需 | 含义 |
|---|---|---|
| `positions` | 是 | `prefix` / `root` / `suffix` 之一或多个。自由词干是 `root`。 |
| `meanings` | 是 | 简短释义，最核心的放最前 |
| `examples` | 否 | 由该词干构成的词；importer 会为它们预计算分解结果 |
| `variants` | 否 | **不规则**的表层异形——见下 |

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
- 不收其他源里已经是能产词缀的形式（`ship`、`ward`、`less`、`hood`、`ment`、`dom`）。
  把它们当词根加进来，搜索会优先选中它们而不是真正的词根。
- 每次新增都要对着 `data/gold/segmentations.tsv` 检查 —— 破坏已有正确分词的词干不收。
