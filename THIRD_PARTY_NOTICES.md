# Third-Party Notices

LexiRoot 的词素数据集以下列第三方项目为起点，之后由本项目自行修改、扩充，并与本项目
原创的词干数据合并成单一文件 `data/sources/morphemes.json`。合并后不再逐条标记来源，
但下列许可证声明适用于这份衍生数据的相应部分——这也是这两个 MIT 项目所要求的全部。

数据集的内容与策展规则见 [`data/README.md`](data/README.md)。

---

## colingoldberg/morphemes

https://github.com/colingoldberg/morphemes

起点：该仓库 `master` 分支 commit `846aa473cb27916f2c3acedb52d98f3a2e2a6572` 的
`data/morphemes.json`（2,435 条，展开为 3,762 个词素）。经本项目修改后并入
`data/sources/morphemes.json`。

```
MIT License

Copyright (c) 2019 Colin Goldberg

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

## WithEnglishWeCan/generated-english-roots-list

https://github.com/WithEnglishWeCan/generated-english-roots-list

起点：该仓库 `master` 分支 commit `bf26e3842137d8f7bbc6e69ef39c05b43e3a22a6` 的
`english.roots.list.build.json`（1,061 条，净增 102 个词素，其余与上一个源重复）。
经本项目修改后并入 `data/sources/morphemes.json`。

该仓库没有单独的 LICENSE 文件，其 README 声明 "Licensed under MIT."，即以 MIT 许可证
授权，完整条款同上。
