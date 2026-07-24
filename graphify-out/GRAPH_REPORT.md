# Graph Report - pixel-game-kit  (2026-07-24)

## Corpus Check
- 45 files · ~90,490 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 712 nodes · 828 edges · 78 communities (28 shown, 50 thin omitted)
- Extraction: 97% EXTRACTED · 3% INFERRED · 0% AMBIGUOUS · INFERRED: 26 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `59e589aa`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- [[_COMMUNITY_Error|Error]]
- [[_COMMUNITY_enabled|enabled]]
- [[_COMMUNITY_detect.rs|detect.rs]]
- [[_COMMUNITY_process_image_common|process_image_common]]
- [[_COMMUNITY_PLAN.md — Pixel Snapper 演进路线|PLAN.md — Pixel Snapper 演进路线]]
- [[_COMMUNITY_Phase 2 Resample Strategies Implementation Plan|Phase 2 Resample Strategies Implementation Plan]]
- [[_COMMUNITY_properties|properties]]
- [[_COMMUNITY_dominant_threshold|dominant_threshold]]
- [[_COMMUNITY_ai-character-cleanup.json|ai-character-cleanup.json]]
- [[_COMMUNITY_properties|properties]]
- [[_COMMUNITY_batch.rs|batch.rs]]
- [[_COMMUNITY_properties|properties]]
- [[_COMMUNITY_properties|properties]]
- [[_COMMUNITY_三、Epic 与 User Story|三、Epic 与 User Story]]
- [[_COMMUNITY_Phase 1 Design Detector Diversity|Phase 1 Design: Detector Diversity]]
- [[_COMMUNITY_Phase 2 Design Resample Strategies|Phase 2 Design: Resample Strategies]]
- [[_COMMUNITY_properties|properties]]
- [[_COMMUNITY_parse_cli_args|parse_cli_args]]
- [[_COMMUNITY_pipeline-config.schema.json|pipeline-config.schema.json]]
- [[_COMMUNITY_Phase 1 Detector Diversity Implementation Plan|Phase 1 Detector Diversity Implementation Plan]]
- [[_COMMUNITY_Sprite Fusion Pixel Snapper|Sprite Fusion Pixel Snapper]]
- [[_COMMUNITY_resample.rs|resample.rs]]
- [[_COMMUNITY_CLAUDE|CLAUDE.md]]
- [[_COMMUNITY_detect_runs|detect_runs]]
- [[_COMMUNITY_detect_tiled|detect_tiled]]
- [[_COMMUNITY_main|main]]
- [[_COMMUNITY_detect.rs|detect.rs]]
- [[_COMMUNITY_apply|apply]]
- [[_COMMUNITY_Config|Config]]
- [[_COMMUNITY_From|From]]
- [[_COMMUNITY_Option|Option]]
- [[_COMMUNITY_Result|Result]]
- [[_COMMUNITY_Self|Self]]
- [[_COMMUNITY_String|String]]
- [[_COMMUNITY_Vec|Vec]]
- [[_COMMUNITY_Config|Config]]
- [[_COMMUNITY_Option|Option]]
- [[_COMMUNITY_RgbaImage|RgbaImage]]
- [[_COMMUNITY_Config|Config]]
- [[_COMMUNITY_Option|Option]]
- [[_COMMUNITY_RgbaImage|RgbaImage]]
- [[_COMMUNITY_Vec|Vec]]
- [[_COMMUNITY_Config|Config]]
- [[_COMMUNITY_Option|Option]]
- [[_COMMUNITY_RgbaImage|RgbaImage]]
- [[_COMMUNITY_Config|Config]]
- [[_COMMUNITY_Option|Option]]
- [[_COMMUNITY_RgbaImage|RgbaImage]]
- [[_COMMUNITY_From|From]]
- [[_COMMUNITY_JsValue|JsValue]]
- [[_COMMUNITY_Result|Result]]
- [[_COMMUNITY_Self|Self]]
- [[_COMMUNITY_String|String]]
- [[_COMMUNITY_Result|Result]]
- [[_COMMUNITY_RgbaImage|RgbaImage]]
- [[_COMMUNITY_Vec|Vec]]
- [[_COMMUNITY_Config|Config]]
- [[_COMMUNITY_Option|Option]]
- [[_COMMUNITY_Result|Result]]
- [[_COMMUNITY_RgbaImage|RgbaImage]]
- [[_COMMUNITY_Vec|Vec]]
- [[_COMMUNITY_Config|Config]]
- [[_COMMUNITY_Result|Result]]
- [[_COMMUNITY_RgbaImage|RgbaImage]]
- [[_COMMUNITY_Config|Config]]
- [[_COMMUNITY_Result|Result]]
- [[_COMMUNITY_RgbaImage|RgbaImage]]
- [[_COMMUNITY_Config|Config]]
- [[_COMMUNITY_Result|Result]]
- [[_COMMUNITY_RgbaImage|RgbaImage]]
- [[_COMMUNITY_Config|Config]]
- [[_COMMUNITY_Result|Result]]
- [[_COMMUNITY_RgbaImage|RgbaImage]]
- [[_COMMUNITY_Config|Config]]
- [[_COMMUNITY_Result|Result]]
- [[_COMMUNITY_Vec|Vec]]
- [[_COMMUNITY_Result|Result]]

## God Nodes (most connected - your core abstractions)
1. `顶层结构` - 27 edges
2. `process_image_common()` - 17 edges
3. `parse_cli_args()` - 15 edges
4. `PLAN.md — Pixel Snapper 演进路线` - 15 edges
5. `Phase 1 Design: Detector Diversity` - 15 edges
6. `Phase 1 Detector Diversity Implementation Plan` - 14 edges
7. `三、Epic 与 User Story` - 13 edges
8. `Phase 3 Quantize Enhancement + Rename Implementation Plan` - 13 edges
9. `Phase 2 Design: Resample Strategies` - 13 edges
10. `Phase 3 Design: Quantize Enhancement + Rename to pixel-game-kit` - 13 edges

## Surprising Connections (you probably didn't know these)
- `BatchConfig` --references--> `顶层结构`  [EXTRACTED]
  src/cli/batch.rs → docs/CONFIG.md
- `detect_elastic()` --references--> `顶层结构`  [EXTRACTED]
  src/detect/elastic.rs → docs/CONFIG.md
- `detect_runs()` --references--> `顶层结构`  [EXTRACTED]
  src/detect/runs.rs → docs/CONFIG.md
- `resample_dominant()` --references--> `顶层结构`  [EXTRACTED]
  src/resample/dominant.rs → docs/CONFIG.md
- `resample_majority()` --references--> `顶层结构`  [EXTRACTED]
  src/resample/majority.rs → docs/CONFIG.md

## Import Cycles
- 1-file cycle: `tests/detect.rs -> tests/detect.rs`

## Communities (78 total, 50 thin omitted)

### Community 0 - "Error"
Cohesion: 0.32
Nodes (5): Display, Formatter, PixelSnapperError, wasm_bindgen::JsValue, ImageError

### Community 1 - "enabled"
Cohesion: 0.05
Nodes (43): additionalProperties, default, properties, type, additionalProperties, default, type, default (+35 more)

### Community 2 - "detect.rs"
Cohesion: 0.10
Nodes (23): Default, apply_palette(), nearest_palette_color(), parse_palette_hex(), Config, Option, Self, String (+15 more)

### Community 3 - "process_image_common"
Cohesion: 0.10
Nodes (35): Error, 顶层结构, detect_elastic(), CutMethod, detect(), DetectionCandidate, DetectStrategy, select_best() (+27 more)

### Community 4 - "PLAN.md — Pixel Snapper 演进路线"
Cohesion: 0.05
Nodes (37): License 合规清单, Phase 0 — 骨架重构（前置，不改行为）✅, Phase 1 — Detector 多样性（最高优先级）✅, Phase 2 — 重采样策略 ✅, Phase 3 — 量化增强 ✅, Phase 4 — 后处理全家桶, Phase 5 — 矢量化（可选，独立）, Phase 6 — 产品功能层（Web + 跨形态共享） (+29 more)

### Community 5 - "Phase 2 Resample Strategies Implementation Plan"
Cohesion: 0.15
Nodes (12): File Structure, Phase 2 Resample Strategies Implementation Plan, Self-Review (completed inline), Task 1: Directory skeleton — move majority, zero behavior change, Task 2: Config resample fields + dispatch on `config.resample_method`, Task 3: median strategy (per-channel median + sample window), Task 4: dominant strategy (dominant color + mean fallback + optional alpha binarize), Task 5: mode strategy (per-channel mode) (+4 more)

### Community 6 - "properties"
Cohesion: 0.07
Nodes (30): DetectConfig, additionalProperties, properties, type, default, description, minimum, type (+22 more)

### Community 7 - "dominant_threshold"
Cohesion: 0.07
Nodes (30): ResampleConfig, additionalProperties, default, properties, type, default, description, maximum (+22 more)

### Community 8 - "ai-character-cleanup.json"
Cohesion: 0.06
Nodes (32): PipelineConfig — 配置规范, 为什么需要它, 命名约定, 字段 ↔ CLI flag ↔ User Story 映射, 待决, 版本策略, 确定性, 调色板优先级 (+24 more)

### Community 9 - "properties"
Cohesion: 0.07
Nodes (29): default, description, enum, default, description, items, type, QuantizeConfig (+21 more)

### Community 10 - "batch.rs"
Cohesion: 0.27
Nodes (15): F, BatchConfig, BatchEvent, collect_batch_inputs(), Config, get_output_path(), is_supported_image_path(), print_processed_image() (+7 more)

### Community 11 - "properties"
Cohesion: 0.05
Nodes (37): additionalProperties, default, description, type, $defs, OutputConfig, description, default (+29 more)

### Community 12 - "properties"
Cohesion: 0.08
Nodes (25): default, $ref, default, $ref, default, $ref, properties, detect (+17 more)

### Community 13 - "三、Epic 与 User Story"
Cohesion: 0.06
Nodes (28): File Structure, Phase 3 Quantize Enhancement + Rename Implementation Plan, Self-Review (completed inline), Task 10: tests/quantize.rs full + CLAUDE.md + final verification, Task 1: Rename to pixel-game-kit + bump 2.0, Task 2: quantize/ directory skeleton (move k-means, zero behavior), Task 3: Config quantize fields, Task 4: Oklab conversion + k-means Oklab distance + flip default (+20 more)

### Community 14 - "Phase 1 Design: Detector Diversity"
Cohesion: 0.07
Nodes (28): fill_holes, remove, remove_floating_pixels, scope, tolerance, description, detect, strategy (+20 more)

### Community 15 - "Phase 2 Design: Resample Strategies"
Cohesion: 0.12
Nodes (22): quantize_kmeans(), Config, Result, RgbaImage, linear_to_srgb(), oklab_to_rgb(), rgb_to_oklab(), srgb_to_linear() (+14 more)

### Community 16 - "properties"
Cohesion: 0.09
Nodes (22): properties, default, description, type, fill_holes, remove, remove_floating_pixels, scope (+14 more)

### Community 17 - "parse_cli_args"
Cohesion: 0.16
Nodes (20): CliCommand, parse_cli_args(), print_cli_help(), Config, ExitCode, Result, String, run_cli() (+12 more)

### Community 18 - "pipeline-config.schema.json"
Cohesion: 0.08
Nodes (25): Epic 10 — 会话与多图管理（完整产品）, Epic 11 — 确定性与可复现（横切铁律 → PLAN R1）, Epic 12 — 错误处理与边缘场景, Epic 1 — 入口与图像输入, Epic 2 — 网格检测与对齐（→ PLAN Phase 1）, Epic 3 — 重采样与抗锯齿（→ PLAN Phase 2）, Epic 4 — 颜色与调色板（→ PLAN Phase 3）, Epic 5 — 后处理（→ PLAN Phase 4） (+17 more)

### Community 19 - "Phase 1 Detector Diversity Implementation Plan"
Cohesion: 0.13
Nodes (14): File Structure, Phase 1 Detector Diversity Implementation Plan, Self-Review (completed inline), Task 10: Fixtures + full integration tests, Task 11: cli.rs split + CLAUDE.md update + final verification, Task 1: detect module skeleton (types + mod declaration, zero behavior), Task 2: elastic detector + Walker pipeline integration (behavior-preserving), Task 3: runs detector (posterize + GCD) (+6 more)

### Community 20 - "Sprite Fusion Pixel Snapper"
Cohesion: 0.17
Nodes (11): Acknowledgments, Brew, Build from source, Cargo, 💻 CLI, License, Perfect for, Sprite Fusion Pixel Snapper (+3 more)

### Community 21 - "resample.rs"
Cohesion: 0.36
Nodes (10): dominant_preserves_sparse_sprite(), each_strategy_produces_deterministic_output(), majority_default_matches_anchor(), manual_method_respected(), median_smooths_aa_edges(), mode_emits_per_channel(), String, sample_window_changes_median_output() (+2 more)

### Community 22 - "CLAUDE.md"
Cohesion: 0.22
Nodes (7): Architecture: dual-target + modular pipeline, Build / test / run, Constraints enforced in code, Determinism, The processing pipeline, Tuning knobs, What this is

### Community 23 - "detect_runs"
Cohesion: 0.09
Nodes (23): Acceptance, Algorithm details, Architecture, Background, CLI, Config (aligns with CONFIG.md `quantize` schema), Data flow, Data Structures (+15 more)

### Community 24 - "detect_tiled"
Cohesion: 0.09
Nodes (23): Acceptance, Architecture, Auto Selection, Background, CLI, Config (aligns with existing schema in CONFIG.md), Data flow (process_image_common change), Data Structures (+15 more)

### Community 26 - "detect.rs"
Cohesion: 0.20
Nodes (11): detect_runs(), pixel_key(), posterize(), auto_picks_correct_detector_per_fixture(), auto_picks_elastic_for_ai_sprite(), elastic_detects_skewed_fixture(), elastic_returns_walker_candidate_for_ai_sprite(), load_fixture() (+3 more)

### Community 27 - "apply"
Cohesion: 0.57
Nodes (6): apply(), apply_threshold(), bayer_matrix(), floyd_steinberg(), RgbaImage, Vec

## Knowledge Gaps
- **354 isolated node(s):** `$schema`, `$id`, `title`, `description`, `type` (+349 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **50 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `顶层结构` connect `process_image_common` to `ai-character-cleanup.json`, `batch.rs`, `detect.rs`?**
  _High betweenness centrality (0.072) - this node is a cross-community bridge._
- **Why does `$defs` connect `properties` to `enabled`, `dominant_threshold`, `properties`, `properties`?**
  _High betweenness centrality (0.065) - this node is a cross-community bridge._
- **Why does `PipelineConfig — 配置规范` connect `ai-character-cleanup.json` to `process_image_common`?**
  _High betweenness centrality (0.060) - this node is a cross-community bridge._
- **Are the 11 inferred relationships involving `process_image_common()` (e.g. with `process_file()` and `detect()`) actually correct?**
  _`process_image_common()` has 11 INFERRED edges - model-reasoned connections that need verification._
- **Are the 10 inferred relationships involving `parse_cli_args()` (e.g. with `parse_palette_hex()` and `output_path_is_required()`) actually correct?**
  _`parse_cli_args()` has 10 INFERRED edges - model-reasoned connections that need verification._
- **What connects `$schema`, `$id`, `title` to the rest of the system?**
  _354 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `enabled` be split into smaller, more focused modules?**
  _Cohesion score 0.047619047619047616 - nodes in this community are weakly interconnected._