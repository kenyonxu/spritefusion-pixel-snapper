# Graph Report - spritefusion-pixel-snapper  (2026-07-23)

## Corpus Check
- 35 files · ~78,177 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 612 nodes · 772 edges · 26 communities (25 shown, 1 thin omitted)
- Extraction: 97% EXTRACTED · 3% INFERRED · 0% AMBIGUOUS · INFERRED: 22 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `0193df6a`
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

## God Nodes (most connected - your core abstractions)
1. `process_image_common()` - 18 edges
2. `PLAN.md — Pixel Snapper 演进路线` - 15 edges
3. `Phase 1 Design: Detector Diversity` - 15 edges
4. `Phase 1 Detector Diversity Implementation Plan` - 14 edges
5. `三、Epic 与 User Story` - 13 edges
6. `Phase 2 Design: Resample Strategies` - 13 edges
7. `parse_cli_args()` - 12 edges
8. `Phase 2 Resample Strategies Implementation Plan` - 12 edges
9. `USER_STORIES.md — Pixel Snatter 用户故事集` - 11 edges
10. `PipelineConfig — 配置规范` - 11 edges

## Surprising Connections (you probably didn't know these)
- `auto_picks_correct_detector_per_fixture()` --calls--> `select_best()`  [INFERRED]
  tests/detect.rs → src/detect/mod.rs
- `auto_picks_elastic_for_ai_sprite()` --calls--> `select_best()`  [INFERRED]
  tests/detect.rs → src/detect/mod.rs
- `parse_cli_args()` --calls--> `parse_palette_hex()`  [INFERRED]
  src/cli/args.rs → src/palette.rs
- `process_file()` --calls--> `process_image_common()`  [INFERRED]
  src/cli/batch.rs → src/lib.rs
- `process_image_common()` --calls--> `detect()`  [INFERRED]
  src/lib.rs → src/detect/mod.rs

## Import Cycles
- 1-file cycle: `tests/detect.rs -> tests/detect.rs`

## Communities (26 total, 1 thin omitted)

### Community 0 - "Error"
Cohesion: 0.06
Nodes (33): Display, Error, Formatter, ImageError, PixelSnapperError, From, JsValue, Result (+25 more)

### Community 1 - "enabled"
Cohesion: 0.05
Nodes (43): additionalProperties, default, properties, type, additionalProperties, default, type, default (+35 more)

### Community 2 - "detect.rs"
Cohesion: 0.08
Nodes (32): Default, Config, Option, Self, String, Vec, detect_elastic(), Config (+24 more)

### Community 3 - "process_image_common"
Cohesion: 0.11
Nodes (32): detect_candidates(), process_image(), process_image_common(), ProcessedImage, Config, JsValue, Option, Result (+24 more)

### Community 4 - "PLAN.md — Pixel Snapper 演进路线"
Cohesion: 0.06
Nodes (35): License 合规清单, Phase 0 — 骨架重构（前置，不改行为）✅, Phase 1 — Detector 多样性（最高优先级）✅, Phase 2 — 重采样策略, Phase 3 — 量化增强, Phase 4 — 后处理全家桶, Phase 5 — 矢量化（可选，独立）, Phase 6 — 产品功能层（Web + 跨形态共享） (+27 more)

### Community 5 - "Phase 2 Resample Strategies Implementation Plan"
Cohesion: 0.08
Nodes (23): PipelineConfig — 配置规范, 为什么需要它, 命名约定, 字段 ↔ CLI flag ↔ User Story 映射, 待决, 版本策略, 确定性, 调色板优先级 (+15 more)

### Community 6 - "properties"
Cohesion: 0.07
Nodes (30): DetectConfig, additionalProperties, properties, type, default, description, minimum, type (+22 more)

### Community 7 - "dominant_threshold"
Cohesion: 0.07
Nodes (30): ResampleConfig, additionalProperties, default, properties, type, default, description, maximum (+22 more)

### Community 8 - "ai-character-cleanup.json"
Cohesion: 0.07
Nodes (28): fill_holes, remove, remove_floating_pixels, scope, tolerance, description, detect, strategy (+20 more)

### Community 9 - "properties"
Cohesion: 0.08
Nodes (26): default, description, enum, default, description, items, type, pattern (+18 more)

### Community 10 - "batch.rs"
Cohesion: 0.20
Nodes (22): F, Path, PathBuf, BatchConfig, BatchEvent, collect_batch_inputs(), Config, get_output_path() (+14 more)

### Community 11 - "properties"
Cohesion: 0.08
Nodes (25): default, description, type, default, description, type, default, description (+17 more)

### Community 12 - "properties"
Cohesion: 0.08
Nodes (25): default, $ref, default, $ref, default, $ref, properties, detect (+17 more)

### Community 13 - "三、Epic 与 User Story"
Cohesion: 0.08
Nodes (25): Epic 10 — 会话与多图管理（完整产品）, Epic 11 — 确定性与可复现（横切铁律 → PLAN R1）, Epic 12 — 错误处理与边缘场景, Epic 1 — 入口与图像输入, Epic 2 — 网格检测与对齐（→ PLAN Phase 1）, Epic 3 — 重采样与抗锯齿（→ PLAN Phase 2）, Epic 4 — 颜色与调色板（→ PLAN Phase 3）, Epic 5 — 后处理（→ PLAN Phase 4） (+17 more)

### Community 14 - "Phase 1 Design: Detector Diversity"
Cohesion: 0.09
Nodes (23): Acceptance, Architecture, Auto Selection, Background, CLI, Config (aligns with existing schema in CONFIG.md), Data flow (process_image_common change), Data Structures (+15 more)

### Community 15 - "Phase 2 Design: Resample Strategies"
Cohesion: 0.09
Nodes (22): Acceptance, Architecture, Background, CLI, Config (aligns with CONFIG.md `resample` schema), Data flow, Data Structures, Decisions (+14 more)

### Community 16 - "properties"
Cohesion: 0.09
Nodes (22): properties, default, description, type, fill_holes, remove, remove_floating_pixels, scope (+14 more)

### Community 17 - "parse_cli_args"
Cohesion: 0.16
Nodes (17): CliCommand, parse_cli_args(), print_cli_help(), Config, ExitCode, Result, String, run_cli() (+9 more)

### Community 18 - "pipeline-config.schema.json"
Cohesion: 0.12
Nodes (15): additionalProperties, $defs, OutputConfig, QuantizeConfig, description, $id, additionalProperties, default (+7 more)

### Community 19 - "Phase 1 Detector Diversity Implementation Plan"
Cohesion: 0.14
Nodes (14): File Structure, Phase 1 Detector Diversity Implementation Plan, Self-Review (completed inline), Task 10: Fixtures + full integration tests, Task 11: cli.rs split + CLAUDE.md update + final verification, Task 1: detect module skeleton (types + mod declaration, zero behavior), Task 2: elastic detector + Walker pipeline integration (behavior-preserving), Task 3: runs detector (posterize + GCD) (+6 more)

### Community 20 - "Sprite Fusion Pixel Snapper"
Cohesion: 0.17
Nodes (11): Acknowledgments, Brew, Build from source, Cargo, 💻 CLI, License, Perfect for, Sprite Fusion Pixel Snapper (+3 more)

### Community 21 - "resample.rs"
Cohesion: 0.42
Nodes (10): dominant_preserves_sparse_sprite(), each_strategy_produces_deterministic_output(), majority_default_matches_anchor(), manual_method_respected(), median_smooths_aa_edges(), mode_emits_per_channel(), String, run_cli() (+2 more)

### Community 22 - "CLAUDE.md"
Cohesion: 0.22
Nodes (7): Architecture: dual-target + modular pipeline, Build / test / run, Constraints enforced in code, Determinism, The processing pipeline, Tuning knobs, What this is

### Community 23 - "detect_runs"
Cohesion: 0.43
Nodes (6): detect_runs(), pixel_key(), posterize(), Config, Option, RgbaImage

### Community 24 - "detect_tiled"
Cohesion: 0.50
Nodes (7): detect_tiled(), gray(), peak_lag(), Config, Option, RgbaImage, stddev()

## Knowledge Gaps
- **310 isolated node(s):** `$schema`, `$id`, `title`, `description`, `type` (+305 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **1 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `$defs` connect `pipeline-config.schema.json` to `enabled`, `properties`, `dominant_threshold`?**
  _High betweenness centrality (0.088) - this node is a cross-community bridge._
- **Why does `properties` connect `properties` to `enabled`, `pipeline-config.schema.json`, `dominant_threshold`?**
  _High betweenness centrality (0.040) - this node is a cross-community bridge._
- **Are the 12 inferred relationships involving `process_image_common()` (e.g. with `process_file()` and `detect()`) actually correct?**
  _`process_image_common()` has 12 INFERRED edges - model-reasoned connections that need verification._
- **What connects `$schema`, `$id`, `title` to the rest of the system?**
  _310 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Error` be split into smaller, more focused modules?**
  _Cohesion score 0.05647840531561462 - nodes in this community are weakly interconnected._
- **Should `enabled` be split into smaller, more focused modules?**
  _Cohesion score 0.047619047619047616 - nodes in this community are weakly interconnected._
- **Should `detect.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.08333333333333333 - nodes in this community are weakly interconnected._