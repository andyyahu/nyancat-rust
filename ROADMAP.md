# Roadmap

這份文件記錄 rustification 進入主線後的後續方向。`RUSTIFICATION_PLAN.md` 保留為已完成階段的歷史紀錄；新的工作以這份 roadmap 和 `RELEASE_CHECKLIST.md` 為準。

## Current State

- `master` 是目前主線，已包含 rustified core。
- `legacy/split-main` 保留 `3544fa8`，作為純拆分後、rustification 前的 baseline。
- `refactor/split-main` 和 `refactor/rustify-core` 是歷史開發分支，可等穩定發行後再決定是否刪除。
- Rustification 階段已取得階段性勝利；後續重心轉向發行品質、可驗證效能、以及更精緻的模組邊界。

## Documentation Map

- `README.md`：使用者入口、基本建置、執行、telnet、benchmark 方法。
- `ARCHITECTURE.md`：目前模組邊界、資料流、runtime policy、performance policy、extension guidelines。
- `RELEASE_CHECKLIST.md`：merge / release candidate 前的硬性驗證流程。
- `BENCHMARKS.md`：本機可重跑的 benchmark snapshot。
- `RUSTIFICATION_PLAN.md`：已完成的 rustification 階段紀錄。
- `ROADMAP.md`：後續工程方向與優先順序。

## Near-Term Plan

### 1. Release Hardening

目標是讓專案從「可用」變成「可以放心發行」。

- 保持 `scripts/release_check.sh` 作為 release gate。
- 讓 release checklist 覆蓋 clean checkout、tagging、artifact、manpage、systemd files。
- 維護 GitHub Actions CI，讓 stable Rust 跑 release check，MSRV job 跑 Rust 1.85.0 test / release build。
- 發行前用 `scripts/benchmark_matrix.sh` 更新 `BENCHMARKS.md`。

完成標準：

- clean working tree 可以一鍵跑完 `scripts/release_check.sh`。
- release tag 前的人工步驟都能在 `RELEASE_CHECKLIST.md` 找到。
- benchmark 宣稱都能回溯到 `BENCHMARKS.md` 的環境與 commit。

### 2. Output Regression Coverage

目前 smoke test 已檢查 byte count 和關鍵輸出 marker，但 byte count 只能證明輸出大小，局部 marker 也不能完整指出所有語意差異。

- 保留 byte count smoke，因為它便宜且能抓到大部分輸出漂移。
- 維護 release smoke 的關鍵 marker 檢查，覆蓋 xterm / truecolor / telnet newline / no-counter 行為。
- 考慮加入小型 golden output fixture，覆蓋 normal、truecolor、telnet newline、crop、benchmark report。
- 若 golden fixture 太脆弱，至少把 smoke output 的關鍵 escape sequence 和 frame marker 做局部檢查。

完成標準：

- render 或 terminal-output 變更可以清楚說明「哪些輸出變了，為什麼合理」。
- release check 能抓到 help / CLI / benchmark / smoke output 的明顯漂移。

### 3. Modern Rust Hardening

目標不是「為了 Rust 而 Rust」，而是務實地處理 C-era 玩具程式常見的 trade off：magic number、sentinel state、raw byte protocol、process-global state、以及靠人工紀律維持的不變條件。每次改動都要能說明它讓程式更安全、更快、或更容易驗證。

- 優先消滅 internal sentinel：用 `Option`、`NonZero*`、newtype、enum 表達狀態，而不是讓 `0`、`-1`、裸整數跨模組傳遞語意。
- 將 public CLI 相容需求限制在 CLI 邊界內；進入 render/runtime/telnet 後應該是已驗證、型別化的設定。
- 將 telnet command / option 從裸 `u8` 逐步收斂成 typed domain，保留未知 option 的 pass-through 能力。
- 強化 Unix FFI 邊界：優先補上 EINTR/error handling 與更清楚的 safe wrapper；若可攜性收益明確，再評估 `libc` / `sigaction` / `signal-hook`。
- 將 render hot path 的改動建立在 benchmark 上；型別化不得引入 per-cell allocation、dynamic dispatch、或高頻 format work。

完成標準：

- 重要 invariants 由型別或建構函式保證，而不是只靠註解或呼叫端紀律。
- legacy compatibility 只留在 parsing/adaptation 層，核心模組不直接依賴 legacy sentinel。
- 每個 safety/refactor commit 都能通過 release gate；效能相關 commit 需重跑 benchmark matrix。

### 4. Performance Discipline

效能優化要建立在可重跑 benchmark 和明確瓶頸上。

- 先 profile，再改 hot path。
- 每次效能相關變更後更新或至少重跑 benchmark matrix。
- 優先觀察 frame buffer reuse、palette lookup、newline conversion、counter formatting、stdout write pattern。
- 避免為了抽象引入 per-cell allocation、dynamic dispatch、或高頻 format work。

完成標準：

- 每個 performance commit 都能附上「變更前 / 變更後」benchmark。
- 若效能沒有明顯改善，保留可讀性收益時才接受。

### 5. Module Elegance

目標不是把檔案切碎，而是讓每個模組的責任更明確。

- `cli.rs` 已有 `OPTION_SPECS`，後續可考慮讓 README / manpage 的 option table 也由同一份資料生成。
- `render.rs` 是最大模組，後續若要拆，優先考慮 `palette`、`frame_buffer`、`render_loop`、`benchmark_stats` 這些自然邊界。
- `telnet.rs` 目前可測性已足夠，除非新增協定行為，否則不急著拆。
- app-level error enum 仍是條件式保留項，只有錯誤語意跨模組變複雜時才做。

完成標準：

- 拆分後測試覆蓋不下降。
- public behavior 和 smoke byte count 沒有非預期變化。
- hot path 不因為拆分而變慢。

### 6. Packaging And Distribution

這部分會把終端玩具推向正式軟體發行品質。

- 確認 `Cargo.toml` metadata 是否足夠支援 crates.io 或 GitHub release。
- 確認 manpage、systemd service、README 安裝步驟一致。
- 考慮提供 release archive checklist，而不是只依賴本機 build。
- 如果要改 default branch 名稱，先同步本機、GitHub default branch、README 文件。

完成標準：

- 新使用者能從 clean checkout 按 README 建置並跑起來。
- 發行者能照 `RELEASE_CHECKLIST.md` 完成 tag / artifact / benchmark / changelog。

## Decision Rules

- 先用 `RELEASE_CHECKLIST.md` 保護行為，再做結構或效能變更。
- 沒有 benchmark 就不宣稱效能進步。
- 沒有輸出驗證就不做大幅 render refactor。
- 新文件要放進 `Documentation Map`，避免散落成無人維護的筆記。
