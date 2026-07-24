# PLAN.md — Pixel Snapper 演进路线

> 以 pixel-game-kit 为内核，分阶段吸收 PixelRefiner 与 unfake.js 的算法优点。
>
> 本文档是开发 backlog，每个 Phase 可独立交付。完成项打勾。

## 选型结论（为什么是 pixel-game-kit）

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

### 前端工程（Phase 6 起引入，与 src/ 并列）

```
web/                        # Vite + React + TypeScript，独立前端项目
  src/
    App.tsx
    store/                  # zustand + persist：会话(images[]) / 历史 / 用户预设
    components/             # shadcn/ui + 自定义：滑块对比、放大镜、调色板编辑器、候选网格选择器
    forms/                  # RJSF 绑 PipelineConfig schema 生成参数表单 + shadcn widget 映射
    worker/                 # Web Worker 封装 WASM process_image（非阻塞，U12.5）
    recipe/                 # PNG zTXt 读写（recipe 嵌入/回填，U11.2）
    wasm-loader.ts          # vite-plugin-wasm 加载 pkg/
  schema/                   # 软链或复制 ../schema/ 供 RJSF 引用
  package.json
```

**重构顺序**：先做零行为的模块拆分（把现有代码搬到上面对应文件，编译通过 + 测试绿），再开始 Phase 1。这一步不算在任何 Phase 里，是"Phase 0：骨架重构"。

---

## Phase 0 — 骨架重构（前置，不改行为）✅

**目标**：把 `lib.rs`（~1460 行）拆成上面结构，所有现有测试仍绿。为后续 Phase 铺路。

**⚠️ 第一步：基线锁定** — 改任何代码前先录两份基线作为「行为零变化」的验收锚点：编译/测试基线 **和** 图像处理行为基线（`cli_tests` 只测参数解析，不覆盖处理输出，必须单独锁，否则搬家时改出行为差异测试仍绿）。
```bash
# (a) 编译 + 测试基线
cargo test 2>&1 | tee .phase0-baseline.log
cargo build --target wasm32-unknown-unknown 2>&1 | tee -a .phase0-baseline.log

# (b) 图像处理行为基线：对每张样本图跑默认配置，录输出 PNG 的 sha256
for img in tests/fixtures/baseline/*.png; do
  stem=$(basename "$img" .png)
  cargo run --release -- "$img" "tests/fixtures/baseline/expected/$stem.png" 16
  sha256sum "tests/fixtures/baseline/expected/$stem.png"
done | tee -a .phase0-baseline.log
# 每步搬家后重跑 (b)，输出 hash 与基线一致才算「行为零变化」
```

- [x] 放样本图到 `tests/fixtures/baseline/`（借 unfake.js `demo-pixel.png` 作为 `ai-sprite.png`，1064×845）
- [x] 基线锁定：`cargo test` + `cargo build --target wasm32` + 样本图输出 sha256 `802857…9f22` 录入 `.phase0-baseline.log`
- [x] 抽 `config.rs`：`Config` + `Default`，`k_seed` 直接重命名为 `seed`（未做 deprecated 别名——`k_seed` 为 private 字段无外部依赖）
- [x] 抽 `validate.rs`：`validate_image_dimensions`
- [x] 抽 `palette.rs`：`parse_palette_hex` + `nearest_palette_color` + `apply_palette` + `MAX_PALETTE_COLORS`
- [x] 抽 `profile.rs`：`compute_profiles` / `estimate_step_size` / `resolve_step_sizes`
- [x] 抽 `stabilize.rs`：`walk` / `stabilize_both_axes` / `stabilize_cuts` / `snap_uniform_cuts` / `sanitize_cuts`
- [ ] ~~建 `detect/mod.rs` + `detect/elastic.rs`~~ → **推迟到 Phase 1**：`walk` 留 `stabilize.rs`，剖面分析留 `profile.rs`；Phase 1 加 runs/tiled 时建 `detect/` 目录
- [ ] ~~建 `resample/mod.rs` + `resample/majority.rs`~~ → **简化为单文件 `resample.rs`**；策略 enum 目录化留 Phase 1
- [ ] ~~建 `quantize/mod.rs` + `quantize/kmeans.rs`~~ → **简化为单文件 `quantize.rs`**；策略 enum 目录化留 Phase 1
- [x] 抽 `cli.rs`（native，整文件 `#![cfg(not(wasm32))]`）；`wasm.rs` **未抽**——`process_image` 留 `lib.rs` 避免 wasm_bindgen 导出可见性变动
- [x] `lib.rs` 只留 `process_image_common` 编排 + `ProcessedImage` + `process_image`(wasm) + `pub use`
- [x] `cargo test` 全绿（5 passed）+ `cargo build --target wasm32` 通过（0 warning）
- [x] 更新 CLAUDE.md 架构章节（双目标 + 模块化管线映射表）

**验收**：✅ 行为零变化，5 个 cli_tests 通过，样本图输出 sha256 `802857…9f22` 全程一致。

### 实施记录

- **分支**：`refactor/phase0-module-split`（commit `c9f45cf` → `8bf2bef` → `1dff622` → `2d01b3d`，未推）
- **结果**：`lib.rs` 1460 → 139 行（-90%），拆为 11 模块（cli/config/error/palette/profile/quantize/resample/stabilize/validate + lib + main）
- **基线纪律生效**：途中两次失误（删 stabilize 段时误删 resample 签名、error.rs 残留 unused `wasm_bindgen` import）都靠 sha256 锚定 + 编译验证即时发现修复
- **可见性调整**：`process_image_common` 与 `ProcessedImage` 改 `pub(crate)` 供 `cli.rs` 访问；`Config` 内部字段改 `pub(crate)`（`k_colors`/`pixel_size_override` 保持 `pub` 供 wasm_bindgen）
- **遗留**：`cli.rs` 553 行偏大（超 400 行 guideline），含入口+批量+测试，Phase 1 可顺手拆 `cli/args.rs` + `cli/batch.rs`

---

## Phase 1 — Detector 多样性（最高优先级）✅

**目标**：打破单一梯度 walker 的弱点。干净图用 runs 最准，复杂背景用 tiled 鲁棒，skew 用 elastic。

**来源映射**：
- `runs.rs` ← unfake.js `crates/unfake-core/src/detect/runs.rs`（逻辑重写，不复制）
- `tiled.rs` ← unfake.js `crates/unfake-core/src/detect/edge.rs`（Sobel + `peak_lag` 自相关）

### 任务

- [x] `detect/runs.rs`：同色 run 长度 → GCD（posterize(64) 预处理抗噪声，← unfake）
- [x] `detect/tiled.rs`：3×3 区块 Sobel + `peak_lag` 自相关 + 投票（← unfake edge.rs）
- [x] `detect/mod.rs`：`DetectStrategy` + `CutMethod` + `DetectionCandidate`（候选 API，spec 决策）+ `select_best`
- [x] `Auto` 调度 → **实现偏差**：非"priority 回退"，改为"全跑 + strong-priority + confidence-fallback"（见实施记录——review 发现 priority-first 会误选）
- [x] Config detect 字段（`detect_strategy` / `runs_min_runs` / `tiled_stddev_threshold` / `tiled_peak_ratio`）
- [ ] ~~`skew_tolerance` 单独字段~~ → **复用 `max_step_ratio`**（elastic 走它，未单独加字段）
- [x] CLI `--detect` + 额外 `--json`（候选列表输出，spec 候选 API）
- [x] WASM `process_image` 加 `detect_strategy` 参数 + 额外 `detect_candidates` 导出（U2.2 Web 候选 UI 铺路）
- [x] 回归测试：`clean.png`（Runs）/`complex-bg.png`（Tiled）/`skewed.png`（Elastic）+ ai-sprite sha256 锚定
- [x] CLAUDE.md pipeline + 模块表更新为多 detector

**额外交付（spec 范围，PLAN 未单列）**：
- [x] `CutMethod` 分流：Uniform → `snap_uniform_cuts`（跳过 walker），Walker → `walk`+`stabilize_both_axes`
- [x] `cli.rs` 拆分 → `cli/{mod,args,batch,cli_tests}.rs`（Phase 0 遗留顺手）

### 验收
- ✅ 三 fixture 命中预期（clean→Runs, complex-bg→Tiled, skewed→Elastic）
- ✅ Auto 选最优（strong-priority + confidence-fallback）
- ✅ 现有行为不变（ai-sprite sha256 `802857…9f22` 锚定保持）

### 风险
- runs 的 GCD 噪声敏感 → ✅ 已用 posterize(64) 预处理缓解
- tiled 的 max_lag=128 上限 → ✅ CLAUDE.md 注明，超大图回退 elastic
- ⚠️ **实施新增**：elastic confidence（峰强度公式）饱和近 1.0，priority-first 会让低置信度 Tiled 压过 Elastic → 已用 strong-priority（runs/tiled conf≥0.6 才优先）+ confidence-fallback 缓解

### 实施记录

- **分支**：`feat/phase1-detectors`（14 commit，含 review fix `b75817c`，已合并 main `b75817c`）
- **结果**：`detect/{mod,elastic,runs,tiled}.rs` + `cli/{mod,args,batch,cli_tests}.rs`（拆分）+ 3 fixtures + `tests/detect.rs`
- **关键 fix（review 发现）**：Auto 选择从 priority-first 改为 **strong-priority + confidence-fallback**——根因是 elastic confidence（峰强度）饱和近 1.0，priority-first 让 low-conf Tiled（0.333）在 ai-sprite（非整数图）压过 Elastic（1.0），破坏 sha256 anchor。同时修了 fallback `max_by` compare 方向反转的 bug
- **spec 增量交付**：候选 API（`DetectionCandidate` + `select_best`，为 U2.2 铺路）、CLI `--json`、WASM `detect_candidates`、`CutMethod` uniform 分流、cli.rs 拆分
- **验证**：14 test passed，wasm 0 warning，ai-sprite sha256 `802857…9f22` 锚定保持
- **遗留**：elastic confidence 标尺偏粗糙（峰强度饱和），未来若 runs/tiled 误判仍多可考虑统一置信度模型；cli.rs 拆分后 args.rs/batch.rs 各 < 400 行 ✓

---

## Phase 2 — 重采样策略 ✅

**目标**：majority 之外给 median（带 AA 去除）、dominant/mode/qvote（抗噪）、content-adaptive（感知最优）。

**来源映射**：
- `median.rs` ← PixelRefiner `src/core/processor.ts` downsample（sampleWindow 思路）
- `dominant.rs` ← unfake.js `downscale.rs`（dominant 阈值 0.15 / mode / qvote）
- `em.rs` ← unfake.js `content_adaptive.rs`（Öztireli-Gross EM，feature gate）

### 任务

- [x] `resample/median.rs`：per-channel median + `sample_window` 邻域，优先 alpha≥16（← PixelRefiner）
- [x] `resample/dominant.rs`：主色占比 ≥ 阈值（0.15）取主色，否则 mean fallback；alpha 二值化可配（默认关）
- [x] `mode` → **单独 `resample/mode.rs`**（非 dominant.rs 内），per-channel mode（caveat：可能组合出新色，文档注明）
- [ ] ~~`dominant.rs` 内 `qvote`~~ → **推迟到 Phase 3**（依赖 Oklab k-means 替代 imagequant）
- [ ] ~~`resample/em.rs` content-adaptive~~ → **推迟**（spec scope 决策：feature gate 单独做，本轮聚焦核心四策略）
- [x] `ResampleMethod { Majority, Median, Dominant, Mode }`（Qvote/ContentAdaptive 推迟）
- [x] Config：`resample_method` / `resample_sample_window` / `resample_dominant_threshold` / `resample_dominant_binarize_alpha`
- [x] CLI `--resample <majority|median|dominant|mode>` + `--sample-window <1-9>`
- [x] 回归测试：`tests/resample.rs`（sha2 跨平台 + temp_dir，23 passed）
- [ ] benchmark（`cargo bench` criterion）→ **未做**（可选，后续单独加）

### 验收
- ✅ median 去 AA（aa-edges fixture + `sample_window` 差异测试）
- ✅ dominant 保边（clean fixture 少色 sprite）
- ✅ 默认 `majority` 零回归（ai-sprite sha256 `802857…9f22` 保持）
- ⏸ EM 感知最优 → 推迟

### 风险
- EM 计算极重 → 推迟（feature gate 单独做）
- qvote imagequant 依赖 → Phase 3 用 Oklab k-means 替代（推迟）
- ⚠️ **实施新增**：`tests/resample.rs` 原 plan 用 `sha256sum` 外部命令 + `/tmp` 字面路径 → Windows 全挂 7 个；已改 `sha2` crate + `std::env::temp_dir`（见实施记录）

### 实施记录

- **分支**：`feat/phase2-resample`（10 commit，含 review fix `da94df6`，已合并 main `da94df6`）
- **结果**：`resample/{mod,majority,median,dominant,mode}.rs` + `aa-edges.png` fixture + `tests/resample.rs`
- **spec 决策落地**：scope 四策略（EM/qvote 推迟）；mode 单独 `mode.rs`（per-channel caveat 注明）；dominant alpha 默认关
- **关键 fix（review 发现）**：`tests/resample.rs` 用 `sha256sum`（Windows 无此命令）+ `/tmp` 字面路径（cargo-test 的 Windows bin 解析为 `C:\tmp`）→ 7 测试 Windows 全挂。改 `sha2` dev-dep + `std::env::temp_dir`，跨平台 23 passed
- **观察**：dominant 在 ai-sprite 上输出 ≈ majority（量化后每 cell top 色普遍超 0.15 阈值，dominant 退化为 top color = majority）—— 符合预期，少色场景才有差异
- **验证**：23 test passed（5 suites），wasm 0 warning，ai-sprite sha256 `802857…9f22` 保持
- **遗留**：benchmark（criterion）未做；EM / qvote 推迟（qvote 待 Phase 3 Oklab）

---

## Phase 3 — 量化增强 ✅

**目标**：Oklab 感知量化 + dithering + 主机调色板。**全程不碰 imagequant**。

**来源映射**：
- `oklab.rs` ← PixelRefiner `src/core/colorUtils.ts`（sRGB→linear→LMS→cbrt→Oklab）
- `kmeans.rs` 改造 ← PixelRefiner `src/core/quantizer.ts` OklabKMeans（随机初始化改 seeded）
- `dither.rs` ← PixelRefiner `quantizer.ts`（FS 7/3/5/1、Bayer 2/4/8、Ordered）
- `palettes.rs` ← PixelRefiner `src/shared/`（11 个 retro palette，纯数据表）

### 任务

- [x] `quantize/oklab.rs`：sRGB↔Oklab 双向转换 + Oklab 平方欧氏距离（← PixelRefiner colorUtils）
- [x] `quantize/kmeans.rs`：`Colorspace` 分支；**Oklab 为默认**（spec 决策：无外部用户，Oklab 默认优于 plan 的 RGB 默认 + opt-in）；RGB 路径 byte-identical 保 v1.x anchor
- [x] `quantize/dither.rs`：FS（7/3/5/1 + strength）+ Bayer 2/4/8 + Ordered —— ⚠️ **bayer 8×8 递归在归一化域（非标准）**，`--dither bayer8` 输出偏，待修
- [x] `quantize/palettes.rs`：**7 真实**（NES deduped 55 / GB / PC-9801 / MSX1 / PICO-8 / Sweetie16 / Endesga32←lospec）+ **SGB/SNES no-op**（无 canonical palette，`palette()` 返 None）
- [x] `Config.quantize`：`colorspace` + `dither` + `dither_strength` + `preset_palette`
- [x] CLI `--colorspace` / `--dither` / `--dither-strength` / `--preset`
- [x] 调色板优先级 → **实现偏差**：`custom`（--palette）> `preset` > k-means（非 plan 的 preset > custom；spec 决策，custom 在 `process_image_common` 后 quantize 覆盖 preset）
- [x] 回归测试：`tests/quantize.rs`（30 passed）；Oklab anchor `3a589ee9…e4420` + RGB anchor `802857…9f22`；PICO-8 preset 颜色约束测试
- [x] **Breaking change 处理**：RGB 兼容路径（`--colorspace rgb` → `802857…9f22`）+ Oklab 默认（`3a589ee9…`）+ **bump 2.0**

**额外交付（spec 范围，PLAN 未单列）**：
- [x] 项目更名 `spritefusion-pixel-snapper` → **`pixel-game-kit`** + bump 2.0（Task 1，已 GitHub repo rename）
- [x] qvote 回填（Phase 2 推迟项）→ `resample/qvote.rs` —— **lite**（whole-pixel vote，≈majority；真正 per-cell Oklab clustering 推迟）
- [x] WASM `process_image` 加 `colorspace` / `dither` / `preset_palette` 参数

### 验收
- ✅ Oklab 默认（渐变平滑）+ RGB 兼容（anchor `802857…9f22` 锁定）
- ✅ dithering 可见可关（FS/Bayer/Ordered 都跑通）
- ✅ 调色板准确（7 真实；PICO-8 颜色约束测试过；SGB/SNES 文档化 no-op）
- ✅ 30 test passed，wasm 0 warning

### 风险
- Oklab 改变输出 → ✅ RGB 兼容（`--colorspace rgb`）+ bump 2.0
- dithering 确定性 → ✅ FS/Bayer 无 RNG（R1 持）
- ⚠️ **实施新增**：bayer8 递归非标准（plan bug）；qvote-lite ≈ majority；SGB/SNES 无 canonical palette —— 前两项已由 cleanup（commit `b027efd`）修复，见下方遗留段

### 实施记录

- **分支**：`feat/phase3-quantize`（10 commit，已合并 main `4f4e691`；GitHub repo 同步 rename `pixel-game-kit`）
- **结果**：`quantize/{mod,kmeans,oklab,dither,palettes}.rs` + `resample/qvote.rs` + Config 字段 + CLI 4 flags + WASM 3 params + `tests/quantize.rs`
- **关键决策**（spec）：Oklab 默认（无外部用户，质量优先）；调色板 custom > preset；更名 pixel-game-kit + 2.0
- **执行方式**：subagent-driven（10 task × implementer，混合手动 review 省 reviewer 额度）；关键 gate（sha256 双锚定 + wasm 0 warning）逐 task 验证
- **遗留 → 已清理**（[cleanup spec](docs/superpowers/specs/2026-07-23-phase3-cleanup-design.md) / [plan](docs/superpowers/plans/2026-07-23-phase3-cleanup.md)，commit `b027efd`，2026-07-23）：
  - ✅ bayer8 递归非标准 → 标准硬编码 8×8 矩阵（0-63 /64）
  - ✅ qvote-lite → 真实 per-cell Oklab k-means（k=4 + vote 最大聚类中心，≠ majority）
  - ✅ native unused-import warning → 已清（native 现仅剩 Cargo 工具链 pdb collision，bin/lib 同名导致，非代码可清）
  - ⏳ 本地目录名仍 `spritefusion-pixel-snapper`（repo rename 不改本地，用户手动 `mv`）
  - ℹ️ SGB/SNES no-op 保留（无 canonical palette，非 bug）

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

## Phase 6 — 产品功能层（Web + 跨形态共享）

**目标**：在算法 Phase 之上构建完整产品，覆盖 USER_STORIES 标「产品」的 story（U9 预设 / U10 会话 / U4.6 调色板编辑器 / U8 导出 / U11.2 recipe）。

**前端栈**：React + Vite + shadcn/ui + RJSF（决策见 [USER_STORIES.md](USER_STORIES.md) 待决问题）。
**核心契约**：所有功能围绕 `PipelineConfig`（见 [docs/CONFIG.md](docs/CONFIG.md)）——RJSF 吃 schema 生成表单，预设/recipe/CLI 共用同一份 JSON。

### 任务

**6A 前端骨架**
- [ ] `web/` Vite + React + TS 初始化 + shadcn/ui 接入
- [ ] `vite-plugin-wasm` 加载 `pkg/pixel_game_kit.js`，异步 loading 态
- [ ] `worker/` Web Worker 封装 `process_image`（postMessage 传 bytes+config，回传 result）—— 非阻塞（U12.5）
- [ ] `forms/` RJSF 绑 `pipeline-config.schema.json` + shadcn widget 映射（调色板→色板选择器、dither→Select、strategy→RadioGroup）
- [ ] 替换根目录 `index.html` 试用页（迁移为 `web/` 起点）

**6B 预设系统（→ U9）**
- [ ] 内置预设（随包读 `schema/presets/*.json`）+ 用户预设（zustand persist → localStorage）
- [ ] 预设列表 UI（shadcn Sidebar）：命名保存 / 加载 / 删除
- [ ] 导入/导出 `.json`（U9.3）
- [ ] CLI `--preset <name|file>` + `--config <file.json>`（Rust serde 读同一 schema）—— CLI/Web 互通（U9.4）
- [ ] 内置场景预设：「AI角色清洗」「Tile去背景」「复古NES风」等（U9.5）

**6C 会话与多图（→ U10）**
- [ ] zustand store：`images[]`（每张含 inputBytes + config + resultUrl + history[]）
- [ ] 多图列表 / 切换 / 删除 / 清空（U10.1/10.3）
- [ ] 处理历史：每张图历次 config+result，点击回退（U10.2）
- [ ] 批量：当前 config 应用到全部 + ZIP 导出（jszip）（U8.3）

**6D 调色板编辑器（→ U4.6/4.7）**
- [ ] 结果调色板可视化（色块网格）
- [ ] 点击色块 → 弹出色板选择器替换 → 写入 `custom_palette` 重跑 quantize
- [ ] 导出 `.hex` / `.gpl` / `.png` 色板文件（U4.7）

**6E 对比与放大镜（→ U7.1/7.2）**
- [ ] 滑块对比（react-compare-slider 或自写）
- [ ] 放大镜：hover 跟随、pixelated 渲染、原图/结果同步位置

**6F 导出（→ U8）**
- [ ] `output.scale` 下拉（1/2/4/8/…/32，最近邻）
- [ ] `output.auto_trim` / `output.force_size` 开关
- [ ] 下载 PNG / 复制到剪贴板 / ZIP 打包

**6G Recipe 追溯闭环（→ U11.2）**
- [ ] Rust core：PNG `zTXt` 读写（嵌入 minified config，键 `pixel-snapper-recipe`）
- [ ] Web：拖入 PNG → 读 recipe → 回填表单
- [ ] CLI `--dump-recipe <png>` 输出 JSON

**6H 跨形态共享**
- [ ] Rust `Config` ↔ `PipelineConfig` JSON serde 双向（Phase 0 重构对齐 snake_case）
- [ ] schema 版本迁移器 `migrate(config, from_v, to_v)`（breaking 时）
- [ ] CI：所有 `schema/presets/*.json` 通过 schema 验证

### 验收
- USER_STORIES 所有 Web story 可点
- 预设 CLI/Web 双向互通（Web 导出 → CLI `--config` 跑出同结果）
- recipe 闭环：处理 → 存 PNG → 拖回 → 表单回填 → 重跑复现
- Web Worker 处理时 UI 不卡（大图 >2s 仍可操作）
- WASM + 前端 bundle 监控（目标 gz < 250KB，不含 EM/vectorize feature）

### 风险
- RJSF 默认表单丑，custom widget 映射工作量大 → 按 story 优先级逐步映射，MVP 先用默认
- WASM 在 Vite 需 `vite-plugin-wasm` + 顶层 await，异步加载要有 loading 态
- bundle 体积：React + shadcn + RJSF 可能 200KB+ gz → code split（调色板编辑器/放大镜 lazy load）
- 预设 schema 演进：靠 version + 迁移器（C 已定）

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
| **MW** | Phase 6 Web 产品（M1 起持续并行） | 双形态落地、产品功能 | 中（前端工程量大） |

M0 → M1 → M2 → M3 → M4 是算法主线；**MW（Phase 6）从 M1 起并行推进**，每个算法 milestone 落地后同步做对应 Web story（如 M1 完成后做候选网格选择器 UI、M2 完成后做调色板/预设 UI）。M5/M6 可插队或跳过。

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
- **为何 imagequant 全程排除**：pixel-game-kit 选为主力的核心理由之一就是 MIT 干净。引入 GPL 会毁掉这个优势。需要工业级量化时走自写 median-cut/octree 或评估许可干净的 Rust crate。
