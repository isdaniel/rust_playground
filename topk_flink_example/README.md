# Top-K System (Rust + Flink)

處理 **DAU ~100M、10M distinct items** 的近似 Top-K 排行系統。支援三種時間窗口查詢：

| 端點 | 窗口 |
|---|---|
| `GET /topk/all_time?k=100` | 從啟動到現在 |
| `GET /topk/last_1m?k=100` | 最近 1 分鐘 |
| `GET /topk/last_5m?k=100` | 最近 5 分鐘 |
| `GET /topk/last_30m?k=100` | 最近 30 分鐘 |
| `GET /topk/last_hour?k=100` | 最近 1 小時 |
| `GET /topk/range?from=<epoch>&to=<epoch>&k=100` | 任意指定區間（分鐘粒度）|

演算法：**Count-Min Sketch (CMS) + 候選堆**。CMS 固定 ~75KB 記憶體就能覆蓋 10M items，誤差 < 0.1%；候選堆（每窗口 5000 個 heavy hitter）負責枚舉可能的 top-K 候選。CMS 可加（additive），所以 custom 區間查詢可以直接把每小時 sketch 逐 cell 相加後重估。

## 架構

```
┌──────────────┐   events    ┌───────┐    ┌─────────┐   CMS + heap   ┌──────────────┐
│ Rust producer├────────────►│ Kafka ├───►│  Flink  ├───────────────►│    Redis     │
│ (Zipf 1.2,   │   (JSON)    │       │    │   job   │  (每 10s flush │ cms:all_time │
│  10M items)  │             │       │    │ (Java)  │   + hour roll) │ cms:hour:{h} │
└──────────────┘             └───────┘    └─────────┘                └──────┬───────┘
                                                                            │
                                                  ┌─────────────────────────┘
                                                  ▼
                                       ┌──────────────────────┐      ┌──────────┐
                                       │ Rust query API       │◄─────┤  client  │
                                       │ (axum :8080,         │ HTTP │  (curl)  │
                                       │  merges hourly CMS)  │      └──────────┘
                                       └──────────────────────┘
```

**資料流**
1. **Producer**（Rust）：用 Zipf 分佈對 10M items 抽樣，以 JSON 寫入 Kafka `events` topic。
2. **Flink job**（Java，parallelism=1）：
   - 消費 Kafka，維護兩個 CMS：全域 `allTime`、當前小時 `hourly`
   - 維護對應的候選 map（每個最多 5000 項，逐出最小估值的 Space-Saving 變體）
   - 每 10 秒把 `allTime` CMS + 候選堆寫 Redis；當 event 的小時和 `currentHour` 不同 → flush 舊 hourly 到 `cms:hour:{h}` 後重置
3. **Query API**（Rust axum）：
   - `all_time` / `last_hour`：直接讀對應 key，以候選堆作 candidate pool，對 CMS 重估後排序回傳 top-K
   - `range`：`MGET` 區間內所有 hourly CMS，逐 cell 相加；union 所有 hourly 候選堆，對合併後的 sketch 重估排序

## 為什麼這樣設計

| 決策 | 理由 |
|---|---|
| CMS + 候選堆（非精確 HashMap）| 10M items × 8 bytes ≈ 80MB/window 還好，但 custom range 無法 merge；CMS 可加 + 固定記憶體 |
| 小時粒度（非分鐘）~~改：分鐘粒度~~ | 可支援 1/5/30 min 任意滑窗；Redis 用 TTL=7d 控制儲存 |
| Flink parallelism = 1 | 若 keyBy(itemId) 則每個 subtask 只看到自己負責的 key，候選堆無法 cross-key 比較；全域 operator 對 75KB sketch 綽綽有餘 |
| 候選堆存在 Redis 而非 Flink state | Query API 不想 RPC 進 Flink；Redis 是 boundary |
| Rust 端不引用 CMS crate | 要跟 Java 對齊 byte layout 和 hash family，自己寫 ~60 行反而可控 |

## CMS 參數推導

目標：ε = 0.001（估計超出真值不超過 ε·N），1−δ = 0.999（置信度）

```
width  w = ⌈e / ε⌉   = ⌈2.718 / 0.001⌉ = 2719
depth  d = ⌈ln(1/δ)⌉ = ⌈ln(1000)⌉      = 7
size     ≈ w·d·4 bytes = 75 KB per sketch
```

**Byte layout**（Rust/Java 共用）：

```
offset 0..4   : width  (u32 little-endian)
offset 4..8   : depth  (u32 little-endian)
offset 8..    : width * depth * u32 LE counters, row-major (row i cell j = 8 + (i*width+j)*4)
```

**Hash family**：`h_i(x) = ((A_i · x + B_i) mod (2^61 − 1)) mod width`，`x = splitmix64(fnv1a_64(item_bytes))`。`HASH_A` / `HASH_B` 兩端列同樣常數。

## 規模分析 (100M DAU)

- 100M DAU × 10 events/user/day = **10^9 events/day ≈ 12K events/sec** 平均
- 假設 3× peak ≈ 36K events/sec；單一 Flink TM (4 core) 做 CMS update 每核心可達 2M+ ops/s → 有 50× headroom
- Redis 儲存（分鐘粒度 + 7 天 TTL）：每分鐘 ~75KB CMS + ~50KB heap ≈ 125KB → **180MB/day**，常駐 ~1.2GB
- Query latency（分鐘粒度 MGET）:
  - `last_1m`：1 次 MGET ≈ 75KB → **< 5ms**
  - `last_5m` / `last_30m`：5 / 30 次 MGET → **5–15ms**
  - `last_hour`：60 次 → **~20ms**
  - 1 天 `range`：1440 次 MGET（~100MB）→ **~200ms**；可加 hourly roll-up sketch 加速

## 啟動

```bash
cd topk_flink_example
docker compose up -d --build
```

等 Flink 起來（~30s）並提交 job：

```bash
docker compose logs -f job-submitter
# 看到 "Job has been submitted" 就可以 Ctrl-C
```

Flink Web UI：<http://localhost:8081>

**Dashboard**：<http://localhost:8080/> — 內建在 query_api 的單頁 UI，可選 window（1m / 5m / 30m / 1h / all_time / custom range）、k、item substring filter、auto-refresh 間隔（1s / 2s / 5s / 15s / off）。每列顯示排名、item、相對長條與估計次數。

查詢（容器外）：

```bash
# 全時段 top 10
curl -s 'http://localhost:8080/topk/all_time?k=10' | jq

# 1 / 5 / 30 分鐘與 1 小時
curl -s 'http://localhost:8080/topk/last_1m?k=10'   | jq
curl -s 'http://localhost:8080/topk/last_5m?k=10'   | jq
curl -s 'http://localhost:8080/topk/last_30m?k=10'  | jq
curl -s 'http://localhost:8080/topk/last_hour?k=10' | jq

# 自訂區間（例如過去 10 分鐘）
NOW=$(date +%s); FROM=$((NOW-600))
curl -s "http://localhost:8080/topk/range?from=${FROM}&to=${NOW}&k=10" | jq
```

回應格式：

```json
[
  {"item": "item_1", "est": 48213},
  {"item": "item_2", "est": 24011},
  ...
]
```

## Source Ingestion Scenarios

Compose 定義了 **4 個 producer service**，每個跑不同模式，用來從多角度驗證系統。
用 compose profile 挑選要啟動哪些：

| Scenario | Profile | 產生的流量 | 主要驗證 |
|---|---|---|---|
| **zipf**（預設）| `zipf` | 穩定 Zipf(1.2) 覆蓋 10M items | `all_time` 頭部穩定為 `item_1..item_10` |
| **burst** | `burst` | 低背景 + 每 30s 5 秒 burst 打在 20 個 hot item | `last_hour` 在 burst 期間會把 hot set 推到頂，`all_time` 則只緩慢移動 |
| **shifting** | `shifting` | 每 2 分鐘換一批 30 個 hot item | `last_hour` 必須追到新熱點，`all_time` 會累積過往所有 hot set |
| **viral** | `viral` | 5 個 item 從 0 逐步放大到佔 90% 流量 | `last_hour` 看它們名次一路爬升；`range` 查過去 vs 現在能看到逆轉 |

**啟動指令**

```bash
# 只跑 baseline (zipf)
docker compose up -d --build

# zipf + burst
docker compose --profile burst up -d --build

# 四個全開 (需要較多 CPU)
docker compose --profile all up -d --build
```

**驗證範例**

```bash
# burst 期間 (開啟 burst profile 後等幾分鐘)
watch -n 2 "curl -s 'http://localhost:8080/topk/last_hour?k=10' | jq -r '.[].item'"
# 會看到 item_1..item_20 間歇性頂到前面

# shifting：觀察 last_hour 每 2 分鐘「換血」
curl -s 'http://localhost:8080/topk/last_hour?k=10' | jq

# viral：對比兩個小時區間，名次變化顯著
NOW=$(date +%s)
curl -s "http://localhost:8080/topk/range?from=$((NOW-600))&to=$((NOW-300))&k=10" | jq  # 5-10 分鐘前
curl -s "http://localhost:8080/topk/range?from=$((NOW-300))&to=${NOW}&k=10" | jq       # 最近 5 分鐘
```

各 producer 用 `--label` 標示自己，看 log：

```bash
docker compose logs -f producer-burst producer-shifting producer-viral
```

## 停止與清理

```bash
docker compose down -v
```

## 精度驗證

啟動後等 producer 跑幾分鐘，從 Redis 抓 ground-truth（producer 端 Zipf 分佈本身就是 ground truth — item_rank=1 必然第一）：

```bash
curl -s 'http://localhost:8080/topk/all_time?k=10' | jq -r '.[].item'
# 預期 item_1..item_10 依序出現；實際順序可能因 hash collision 微動
```

Top-100 overlap ratio 在 Zipf s=1.2 / 10M items / 100K events 下通常 > 95%。

## 非目標

- 不做 exactly-once（至少一次語意，CMS 對重送有鎖緊 saturating_add 保護）
- Flink state backend 採 local filesystem，沒設 S3/GCS checkpoint
- Producer 單 instance；想壓大流量可 `docker compose up --scale producer=8`
- Hourly sketch 不自動 roll-up 成 daily（production 應加以降低 range query MGET 次數）
- 候選堆 5000 容量對 k > 500 的查詢可能漏（增加 `CANDIDATE_CAPACITY` 即可）

## 檔案對照

| 檔案 | 用途 |
|---|---|
| `common/src/lib.rs` | CountMinSketch、Event、Redis key 慣例、hash family |
| `producer/src/main.rs` | Zipf 事件產生器（rdkafka）|
| `query_api/src/main.rs` | axum HTTP endpoints、range 合併邏輯 |
| `flink_job/src/main/java/com/example/topk/TopKJob.java` | Flink streaming job |
| `flink_job/pom.xml` | Maven 打 fat jar |
| `docker-compose.yml` | Zookeeper / Kafka / Redis / Flink JM+TM / job submitter / producer / query API |
