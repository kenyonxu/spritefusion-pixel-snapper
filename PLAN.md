# PLAN.md — Pixel Snapper 演进路线

> 以 spritefusion-pixel-snapper 为内核，分阶段吸收 PixelRefiner 与 unfake.js 的算法优点。
>
> 本文档是开发 backlog，每个 Phase 可独立交付。完成项打勾。

## 选型结论（为什么是 spritefusion）

1. **唯一有算法护城河**：弹性 walker + skew 检测，是三者中唯一能处理非整数倍、倾斜网格的。另两个都假设整数倍 scale。
2. **确定性（seed=42）**：每个改动可回归测试（同图同参 → byte 一致）。
3. **License 干净**：MIT，零 GPL。可自由移植另两个的算法，不会被 imagequant 的 GPL 污染。
4. **单 crate 干净 + 双目标就绪**：CLI/WASM 已通，扩展自由。

## 铁律（每个 Phase 都要遵守）

- **R1 确定性**：所有 RNG 走 `ChaCha8Rng::seed_from_u64(config.seed)`（默认 42）。移植 PixelRefiner 的 K-means 时把 `Math.random` 换成 seeded。
- **R2 双目标**：新增代码在 native + wasm32 都编译。CLI/文件系统代码用 `#[cfg(not(target_arch="wasm32"))]`，WASM 导出走 `#[wasm_bindgen]`。
- **R3 无 GPL**：只移植**自写算法逻辑**（重写为 Rust）。**绝不**拉 `imagequant` binding。矢量化 tracing 不内嵌 `imagetracer.js`。
- **R4 小文件**：打破 `lib.rs`，按 pipeline 阶段拆模块（参考"目标结构"）。单文件 < 400 行。
- **R5 可插拔**：每个 stage 做成策略 enum，保留现有行为为默认，新策略并列加入。
- **R6 回归测试**：每个新算法加 `tests/fixtures/` 样本 + 输出 hash 错配测试。`seed` 保证可复现。

## 目标文件结构

```
src/
  lib.rs              # process_image_common 编排 + 公共类型/错误
  config.rs           # 分模块 Config（detect/resample/quantize/postprocess/seed）
  validate.rs         # 尺寸/参数校验（从 lib.rs 抽出）
  palette.rs          # parse_palette_hex + nearest_palette_color + 主机调色板预设
  detect/
    mod.rs            # DetectStrategy enum + auto 调度
    elastic.rs        # 现有梯度 walker（从 lib.rs 迁出）
    runs.rs           # [P1] runs GCD detector（← unfake）
    tiled.rs          # [P1] tiled Sobel + 自相关 detector（← unfake）
  resample/
    mod.rs            # ResampleMethod enum + 调度
    majority.rs       # 现有 majority-vote（从 lib.rs 迁出）
    median.rs         # [P2] per-channel median + AA 去除（← PixelRefiner）
    dominant.rs       # [P2] dominant/mode/qvote（← unfake）
    em.rs             # [P2, feature] Öztireli-Gross content-adaptive（← unfake）
  quantize/
    mod.rs            # QuantizeConfig 调度
    oklab.rs          # [P3] sRGB↔Oklab 转换（← PixelRefiner colorUtils.ts）
    kmeans.rs         # 现有 k-means 迁到此，改 Oklab 距离
    dither.rs         # [P3] Floyd-Steinberg / Bayer / Ordered（← PixelRefiner）
    palettes.rs       # [P3] NES/GB/SNES/PICO-8 等主机调色板数据（← PixelRefiner shared/）
  postprocess/
    mod.rs            # PostConfig 调度
    floodfill.rs      # [P4] 背景 flood-fill + floating-island 清理（← PixelRefiner）
    outline.rs        # [P4] 描边 8-way/4-way（← PixelRefiner outline.ts）
    morphology.rs     # [P4] morph open/close 2×2（← unfake morphology.rs）
    alpha.rs          # [P4] alpha 二值化
  stabilize.rs        # 现有 stabilize_both_axes/stabilize_cuts/snap_uniform_cuts/sanitize_cuts
  profile.rs          # 现有 compute_profiles / estimate_step_size / resolve_step_sizes
  cli.rs              # [native] run_cli + parse_cli_args + batch（从 lib.rs 迁出）
  wasm.rs             # [wasm] process_image 导出（从 lib.rs 迁出）
  main.rs             # 不变，7 行 shim
```

**重构顺序**：先做零行为的模块拆分（把现有代码搬到上面对应文件，编译通过 + 测试绿），再开始 Phase 1。这一步不算在任何 Phase 里，是"Phase 0：骨架重构"。

---

## Phase 0 — 骨架重构（前置，不改行为）

**目标**：把 `lib.rs`（~1460 行）拆成上面结构，所有现有测试仍绿。为后续 Phase 铺路。

- [ ] 抽 `config.rs`：`Config` 结构 + `Default` impl，新增 `seed: u64` 字段（默认 42），现有 `k_seed` 标 deprecated 别名
- [ ] 抽 `validate.rs`：`validate_image_dimensions` + pixel_size_override 校验
- [ ] 抽 `palette.rs`：`parse_palette_hex` + `nearest_palette_color` + `apply_palette`
- [ ] 抽 `profile.rs`：`compute_profiles` / `estimate_step_size` / `resolve_step_sizes`
- [ ] 抽 `stabilize.rs`：`walk` / `stabilize_both_axes` / `stabilize_cuts` / `snap_uniform_cuts` / `sanitize_cuts`
- [ ] 建 `detect/mod.rs` + `detect/elastic.rs`：把 walker 相关逻辑迁入，`DetectStrategy::Elastic` 作为当前唯一策略
- [ ] 建 `resample/mod.rs` + `resample/majority.rs`：`ResampleMethod::Majority`
- [ ] 建 `quantize/mod.rs` + `quantize/kmeans.rs`：现有 k-means
- [ ] 抽 `cli.rs`（native）/ `wasm.rs`（wasm）：入口分离
- [ ] `lib.rs` 只留 `process_image_common` 编排 + `pub use` 重导出
- [ ] `cargo test` 全绿 + `cargo build --target wasm32-unknown-unknown` 通过
- [ ] 更新 CLAUDE.md 的架构章节反映新结构

**验收**：行为零变化，现有 5 个 cli_tests 通过，输出 byte 不变。

---

## Phase 1 — Detector 多样性（最高优先级）

**目标**：打破单一梯度 walker 的弱点。干净图用 runs 最准，复杂背景用 tiled 鲁棒，skew 用 elastic。

**来源映射**：
- `runs.rs` ← unfake.js `crates/unfake-core/src/detect/runs.rs`（逻辑重写，不复制）
- `tiled.rs` ← unfake.js `crates/unfake-core/src/detect/edge.rs`（Sobel + `peak_lag` 自相关）

### 任务

- [ ] `detect/runs.rs`：水平/垂直同色 run 长度收集 → GCD → 返回整数 scale；run 总数 < 阈值返回 None
- [ ] `detect/tiled.rs`：3×3 区块（25% 重叠）→ 过滤 stddev < 5 平坦块 → Sobel → 剖面自相关（`peak_lag`，max_lag clamp min(n/8,128)，0.6·gmax 阈值）→ mode 投票
- [ ] `detect/mod.rs`：`DetectStrategy { Auto, Runs, Tiled, Elastic }`
- [ ] `Auto` 调度：runs 优先（>1 即用）→ 回退 tiled → 都失败用 elastic
- [ ] `Config.detect.strategy` + 各 detector 参数（runs_min_runs、tiled_stddev_threshold、tiled_peak_ratio）
- [ ] `Config` 加 `skew_tolerance: Option<f64>`：elastic 模式下走 `max_step_ratio`，runs/tiled 命中整数倍时跳过 walker 直接进 resample
- [ ] CLI 加 `--detect <auto|runs|tiled|elastic>`（默认 auto）
- [ ] WASM `process_image` 加 `detect_strategy` 参数
- [ ] 回归测试：`tests/fixtures/clean_sprite.png`（runs 应命中）、`fixtures/complex_bg.png`（tiled 应命中）、`fixtures/skewed.png`（elastic 应命中）
- [ ] 文档：CLAUDE.md 的 pipeline 第 3-4 步更新为多 detector

### 验收
- 三种 fixture 各自命中预期 detector
- Auto 模式在每张 fixture 上选出最优 detector
- 现有行为（不指定 strategy）输出不变

### 风险
- runs 的 GCD 对单像素噪声敏感（一行 off-by-one → GCD=1）→ 加预处理：检测前轻量 posterize 或众数平滑（参考 PixelRefiner `posterize(img,64)`）
- tiled 的 max_lag=128 上限检测不到超大 scale → 文档注明限制，超大图回退 elastic

---

## Phase 2 — 重采样策略

**目标**：majority 之外给 median（带 AA 去除）、dominant/mode/qvote（抗噪）、content-adaptive（感知最优）。

**来源映射**：
- `median.rs` ← PixelRefiner `src/core/processor.ts` downsample（sampleWindow 思路）
- `dominant.rs` ← unfake.js `downscale.rs`（dominant 阈值 0.15 / mode / qvote）
- `em.rs` ← unfake.js `content_adaptive.rs`（Öztireli-Gross EM，feature gate）

### 任务

- [ ] `resample/median.rs`：per-channel median，`sample_window` 邻域，优先 alpha≥16 不透明像素
- [ ] `resample/dominant.rs`：HashMap 计数，主色占比 ≥ 阈值（默认 0.15）取主色，否则 mean fallback；alpha 硬二值化（可配）
- [ ] `resample/dominant.rs` 内补 `mode` / `qvote`（qvote 先 imagequant-free 量化再投票——用 P3 的 Oklab k-means 替代 imagequant）
- [ ] `resample/em.rs`（feature `content-adaptive`，默认关）：Öztireli-Gross 5 次 EM 迭代，sRGB/D65 Lab 空间，Gaussian 加权 GMM，`clamp_covariance` 特征值 [0.5, 0.5·r_avg]，RATIO_CAP=3 预缩放
- [ ] `ResampleMethod { Majority, Median, Dominant, Mode, Qvote, ContentAdaptive }`
- [ ] `Config.resample.method` + `sample_window` + `dominant_threshold`
- [ ] CLI `--resample <...>` + `--sample-window <n>`
- [ ] 回归测试：每策略一个 fixture + hash
- [ ] benchmark：`cargo bench`（criterion）记录各策略耗时，EM 单列

### 验收
- median 在抗锯齿图上去 AA 效果优于 majority
- dominant 在少色 sprite 上保边
- EM 在感知质量上最优但耗时 10×+（文档标注）

### 风险
- EM 计算极重 → 必须 feature gate，默认 WASM 不启用，防拖胖体积
- qvote 原依赖 imagequant → 用 P3 Oklab k-means 替代，结果非 bit-exact 但避 GPL

---

## Phase 3 — 量化增强

**目标**：Oklab 感知量化 + dithering + 主机调色板。**全程不碰 imagequant**。

**来源映射**：
- `oklab.rs` ← PixelRefiner `src/core/colorUtils.ts`（sRGB→linear→LMS→cbrt→Oklab）
- `kmeans.rs` 改造 ← PixelRefiner `src/core/quantizer.ts` OklabKMeans（随机初始化改 seeded）
- `dither.rs` ← PixelRefiner `quantizer.ts`（FS 7/3/5/1、Bayer 2/4/8、Ordered）
- `palettes.rs` ← PixelRefiner `src/shared/`（11 个 retro palette，纯数据表）

### 任务

- [ ] `quantize/oklab.rs`：sRGB↔Oklab 双向转换 + Oklab 平方欧氏距离
- [ ] 改造 `quantize/kmeans.rs`：距离从 RGB 迁到 Oklab；`Config.quantize.colorspace { Rgb, Oklab }`（Rgb 为默认保旧行为）
- [ ] `quantize/dither.rs`：Floyd-Steinberg（7/3/5/1，strength 可调，跳 alpha=0）、Bayer 2/4/8、Ordered；`DitherMethod { None, FloydSteinberg, Bayer(N), Ordered }`
- [ ] `quantize/palettes.rs`：NES / GameBoy / SGB / SNES / PC-9801 / MSX1 / PICO-8 / Sweetie16 / Endesga 等；`PresetPalette` enum
- [ ] `Config.quantize`：`colorspace` + `dither` + `dither_strength` + `preset_palette: Option<PresetPalette>`
- [ ] CLI `--colorspace <rgb|oklab>` + `--dither <...>` + `--preset <nes|gb|...>`
- [ ] 固定调色板路径：preset 优先 > 自定义 hex > k-means
- [ ] 回归测试：Oklab vs RGB 输出对比 fixture；每个 dither 一个 fixture；每个 preset 一个 fixture
- [ ] **Breaking change 处理**：Oklab 迁移改变默认输出 → 保留 RGB 为默认，Oklab 显式开启；或 bump major version（2.0）

### 验收
- Oklab 量化在渐变图上色带比 RGB 平滑
- dithering 可见且可关
- 主机调色板准确（与原始主机色值对比）

### 风险
- Oklab 改变输出 → 当 breaking change，加 colorspace 配置兼容
- dithering 破坏确定性？不会——FS/Bayer 都是无 RNG 的确定过程，满足 R1

---

## Phase 4 — 后处理全家桶

**目标**：背景去除、描边、形态学、alpha 二值化，让输出可直接用于游戏引擎。

**来源映射**：
- `floodfill.rs` ← PixelRefiner `floodfill.ts` + `processor.ts`（floating-island 清理）
- `outline.rs` ← PixelRefiner `outline.ts`
- `morphology.rs` ← unfake.js `morphology.rs`（2×2 open→close）
- `alpha.rs` ← unfake.js（阈值二值化，但加 Otsu 自适应选项超越原版）

### 任务

- [ ] `postprocess/floodfill.rs`：栈式 flood-fill（非递归），按通道绝对差容差，4/8 连通，scope `Outer/Selected/All`
- [ ] `postprocess/floodfill.rs` 内 `remove_small_floating_components`：BFS 连通分量，小于阈值且非最大分量置零
- [ ] `postprocess/outline.rs`：扩 1px canvas，扫描透明像素，rounded=8 邻域 / sharp=4 邻域，单像素厚度
- [ ] `postprocess/morphology.rs`：2×2 kernel open→close（erode=min/dilate=max），per-channel，replicate border
- [ ] `postprocess/alpha.rs`：固定阈值（默认 128）+ Otsu 自适应（超越 unfake 的硬阈值）
- [ ] `Config.postprocess`：`bg_removal` / `outline` / `morph` / `alpha_binarize` 各自开关 + 参数
- [ ] CLI `--bg-remove` / `--outline <rounded|sharp>` / `--morph` / `--alpha-threshold <n|auto>`
- [ ] 回归测试：带背景 fixture、描边 fixture、噪点 fixture

### 验收
- 背景去除保留主体、清孤立噪点
- 描边单像素、8/4 向正确
- morph 填 2×2 孔、去单像素 speckle
- alpha Otsu 在半透明边缘图上优于硬阈值

---

## Phase 5 — 矢量化（可选，独立）

**目标**：raster → SVG。**只移植预处理滤波，tracing 留前端**。

**来源映射**：
- 预处理 ← unfake.js `vector.rs`（bilateral / median / morph_close_k / gaussian）
- tracing ← 不内嵌，文档示例用前端 `imagetracer.js`

### 任务

- [ ] `postprocess/filters.rs`（feature `vectorize`）：bilateral（sigma_color=d·2, sigma_space=d/2）、median K×K、gaussian separable（σ=0.3·((k-1)·0.5-1)+0.8）
- [ ] `vectorize/mod.rs`：`prepare_for_trace(img, filter, bg)` → 输出预处理后的 RGBA
- [ ] WASM 导出 `prepare_vectorize`，JS 侧调 imagetracer.js
- [ ] 文档：`docs/vectorize.md` 给前端集成示例（imagetracer.js）
- [ ] 评估 Rust `tracer` crate 作纯 Rust 备选（可选，若许可干净）

### 验收
- 预处理后 SVG 描边比直trace 干净
- WASM 体积增量 < 50KB（feature 关时为 0）

### 风险
- 评估 `tracer` crate 许可（避开 GPL）
- feature gate 必须严格，默认关

---

## 跨 Phase：测试与质量

- [ ] `tests/fixtures/` 建立样本库：clean / complex-bg / skewed / aa-edges / gradient / transparent-bg / noisy
- [ ] 每个算法至少一个 fixture + 输出 PNG + hash 锁定测试
- [ ] `cargo bench`（criterion）：各 detector / resample / 量化策略的 p50 耗时
- [ ] WASM 体积监控：CI 记录 `pkg/*.wasm` 大小，feature gate 后回归
- [ ] `cargo clippy -- -D warnings` 进 CI
- [ ] 确定性测试：同图同参跑 2 次，assert byte 一致（覆盖 R1）

## 推荐执行顺序与里程碑

| 里程碑 | 内容 | 价值 | 风险 |
|--------|------|------|------|
| **M0** | Phase 0 骨架重构 | 为一切铺路 | 低（纯搬家） |
| **M1** | Phase 1 detector 多样性 | 修补最大弱点，覆盖面↑ | 低 |
| **M2** | Phase 3 Oklab + 主机调色板 | 量化质量↑，用户可见 | 中（breaking） |
| **M3** | Phase 2 resample 策略 | 抗 AA / 抗噪 | 低 |
| **M4** | Phase 4 后处理 | 游戏引擎可用性 | 低 |
| **M5** | Phase 3 dithering | 复古风 | 低 |
| **M6** | Phase 5 矢量化 | 新能力 | 中 |

M1 → M2 → M3 → M4 是主线；M5/M6 可插队或跳过。

## License 合规清单

| 可移植（重写为 Rust） | 不可拉依赖/内嵌 |
|----------------------|----------------|
| ✅ runs / tiled detector（← unfake） | ❌ imagequant / libimagequant（GPL-3.0） |
| ✅ Öztireli EM content-adaptive（← unfake） | ❌ imagetracer.js 内嵌（留前端） |
| ✅ morph open/close（← unfake） | |
| ✅ Oklab + K-means（← PixelRefiner） | |
| ✅ dithering FS/Bayer/Ordered（← PixelRefiner） | |
| ✅ 主机调色板数据（← PixelRefiner，事实数据） | |
| ✅ flood-fill / outline（← PixelRefiner） | |
| ✅ bilateral/median/gaussian 滤波（← unfake） | |

**规则**：算法逻辑 clean-room 重写，不复制源码。README/NOTICE 注明灵感来源。三个工程均 MIT，逻辑移植合法。

## 决策记录

- **为何不 fork unfake 做底座**：imagequant 的 GPL 会传染；矢量化依赖 JS 层；3-crate 上手成本高。
- **为何不 fork PixelRefiner**：TS 性能 ceiling；`Math.random` 非确定；`processor.ts` 2432 行技术债；无 CLI/WASM。
- **为何 imagequant 全程排除**：spritefusion 选为主力的核心理由之一就是 MIT 干净。引入 GPL 会毁掉这个优势。需要工业级量化时走自写 median-cut/octree 或评估许可干净的 Rust crate。
