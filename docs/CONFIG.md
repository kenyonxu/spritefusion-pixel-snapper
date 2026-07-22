# PipelineConfig — 配置规范

> 单一真相源：CLI、Web、预设文件、配方追溯共用同一份 JSON schema。
>
> Schema 规范：[../schema/pipeline-config.schema.json](../schema/pipeline-config.schema.json) · 示例预设：[../schema/presets/](../schema/presets/)

## 为什么需要它

USER_STORIES 里浮现的问题：CLI 的 flags、Web 的表单、导出的预设文件、嵌入 PNG 的配方——四样东西各写一份就会漂移。`PipelineConfig` 是它们共同的契约：

- **CLI**：`--config presets/xxx.json` 加载；每个 flag 是某个字段的快捷方式
- **Web**：表单双向绑定到 config 对象；导入/导出就是这个 JSON
- **预设**（U9）：命名的 config 文件，团队共享
- **配方追溯**（U11.2）：config 嵌入输出 PNG，任何资产可反查怎么来的

## 顶层结构

```
PipelineConfig
├── version      (固定 1，schema 版本)
├── seed         (全局确定性种子，默认 42)
├── detect       (网格检测 → PLAN P1)
├── resample     (重采样 → PLAN P2)
├── quantize     (颜色/调色板/dithering → PLAN P3)
├── postprocess  (背景/描边/形态学/alpha → PLAN P4)
└── output       (缩放/裁边/配方嵌入)
```

每个子对象所有字段可选，缺省用 `default`。空对象 `{}` = 全默认。

## 字段 ↔ CLI flag ↔ User Story 映射

| 字段 | CLI flag | 默认 | User Story |
|------|---------|------|-----------|
| `seed` | `--seed <N>` | 42 | U11.1, U11.3 |
| `detect.strategy` | `--detect <auto\|runs\|tiled\|elastic>` | auto | U2.1, U2.5 |
| `detect.pixel_size_override` | `--pixel-size <N>` | null | U2.3 |
| `detect.skew_tolerance` | `--skew-tolerance <N>` | 1.8 | U2.4 |
| `resample.method` | `--resample <...>` | majority | U3.1–3.3 |
| `resample.sample_window` | `--sample-window <N>` | 3 | U3.2 |
| `quantize.enabled` | `--no-quantize` | true | U4.1 |
| `quantize.k_colors` | 位置参数 `[COLORS]` | 16 | U4.1 |
| `quantize.colorspace` | `--colorspace <rgb\|oklab>` | rgb | U4.2 |
| `quantize.dither.method` | `--dither <...>` | none | U4.5 |
| `quantize.preset_palette` | `--preset <nes\|gb\|...>` | none | U4.3 |
| `quantize.custom_palette` | `--palette <hex,...>` | null | U4.4 |
| `postprocess.background.remove` | `--bg-remove` | false | U5.1 |
| `postprocess.background.scope` | `--bg-scope <...>` | outer | U5.1 |
| `postprocess.outline.enabled` | `--outline <rounded\|sharp>` | false | U5.4 |
| `postprocess.morphology.enabled` | `--morph` | false | U5.3 |
| `postprocess.alpha.binarize` | `--alpha-threshold <N\|auto>` | false | U5.5 |
| `output.scale` | `--scale <N>` | 1 | U8.2 |
| `output.auto_trim` | `--auto-trim` | false | U8.4 |
| `output.force_size` | `--force-size <WxH>` | null | U8.5 |
| `output.embed_recipe` | `--no-recipe` | true | U11.2 |
| （整文件） | `--config <file.json>` / `--preset <name>` | — | U9.1–9.4 |

**优先级**：`--config` 文件 < 命令行 flag（flag 覆盖文件）。这样预设可被单次 flag 微调。

## 调色板优先级

`custom_palette` > `preset_palette` > `k_colors`(k-means)。三者并存时，前者生效，后者忽略。这与现状（`--palette` 覆盖 k-means）一致。

## 确定性

- 所有 RNG 走 `ChaCha8Rng::seed_from_u64(seed)`（PLAN 铁律 R1）。
- 同输入 + 同 config（含 seed）→ byte 一致输出。
- recipe 嵌入让"同输出"可验证：从 PNG 取出 config，重跑应复现。

## 配方嵌入（Recipe Embedding）

`output.embed_recipe: true` 时，把 minified config 写入输出 PNG 的 `zTXt` chunk：

- **key**：`pixel-snapper-recipe`
- **value**：deflate 压缩的 minified JSON
- **格式**：`{"v":1,"seed":42,"detect":{...},...}`（字段名缩短省空间）

任何输出 PNG 都可反查完整处理参数。CLI 加 `--dump-recipe <png>` 读出；Web 拖入 PNG 自动回填表单。

> 用 `zTXt` 而非 `tEXt`：config 可能较长，压缩避免体积膨胀。PNG spec 原生支持。

## 版本策略

- 顶层 `version` 固定 `1`。minor/additive 变更（加字段、加枚举值、改默认）**不递增**——靠默认值向后兼容。
- breaking 变更（删字段、改语义、改类型）→ bump `version` + 写迁移器 `migrate_v1_v2(config)`。
- Rust 侧用 serde + `#[serde(default)]` 实现"缺字段填默认"，新旧 config 互通。
- recipe 里的 `v` 字段让嵌入的配方可识别版本并按需迁移。

## 命名约定

- **snake_case**：JSON 字段全用 snake_case，与 CLI flag（`--kebab-case`）一一对应（`-` ↔ `_`）。
- 枚举值用 snake_case 字符串（`floyd_steinberg`、`content_adaptive`），不用 SCREAMING_CASE。
- Rust serde 用 `#[serde(rename_all = "snake_case")]` 自动对齐 enum。

## 验证

- JSON Schema Draft 2020-12，可用任何标准 validator（`ajv-cli`、`jsonschema`）。
- Rust 侧：考虑 `schemars` 从类型生成 schema（保证代码与 schema 不漂移），或手写 + 测试锁定。
- CI 加一步：所有 `schema/presets/*.json` 必须通过 schema 验证。

## 待决

1. **Rust 类型 ↔ schema 同步方式**：手写 schema + 测试 vs `schemars` 自动生成。前者控制力强易漂移，后者自动化但生成 schema 可读性差。建议 Phase 0 重构时定。
2. **Web 端表单库**：基于此 schema 可用 `react-jsonschema-form` / `formsnap` 自动生成表单——这是 **B（前端栈）** 决策的输入之一。
3. **预设命名空间**：内置预设（随包发布）vs 用户预设（本地）。需要在 Phase 6（产品层）定义存储与发现机制。
