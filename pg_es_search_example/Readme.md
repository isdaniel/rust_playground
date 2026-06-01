# pg_es_search_example 架構說明

## 專案概述

本專案展示如何結合 **PostgreSQL** 與 **Elasticsearch** 建構一個商品目錄 API，實現高效的模糊搜尋功能。採用 **Actix-web** 作為 Web 框架，並透過 **Docker Compose** 一鍵啟動所有服務。

## 架構圖

```
                         ┌─────────────┐
                         │   Client    │
                         │  (curl/UI)  │
                         └──────┬──────┘
                                │ HTTP
                                ▼
                      ┌───────────────────┐
                      │   Actix-web API   │
                      │   (Rust App)      │
                      │   Port: 8080      │
                      └───┬──────────┬────┘
                          │          │
            ┌─────────────┤          ├─────────────┐
            │  寫入/讀取   │          │  搜尋/索引   │
            ▼              │          ▼              │
   ┌─────────────────┐    │   ┌─────────────────┐   │
   │   PostgreSQL    │    │   │ Elasticsearch   │   │
   │   Port: 5432    │    │   │   Port: 9200    │   │
   │                 │    │   │                 │   │
   │  - 資料主儲存   │    │   │  - 模糊搜尋     │   │
   │  - ACID 交易    │    │   │  - 全文檢索     │   │
   │  - 資料一致性   │    │   │  - 前綴匹配     │   │
   └─────────────────┘    │   └─────────────────┘   │
                          │                          │
                          └──────────────────────────┘
```

## 為什麼同時使用 PostgreSQL 與 Elasticsearch？

| 特性 | PostgreSQL | Elasticsearch |
|------|-----------|---------------|
| 資料一致性 | ACID 交易保證 | 最終一致性 |
| 精確查詢 | 高效 (索引) | 可以但非最佳用途 |
| 模糊搜尋 | `LIKE`/`ILIKE` 效能差 | 原生支援，高效 |
| 全文檢索 | 基本支援 | 進階分詞、相關性評分 |
| 拼寫容錯 | 不支援 | 原生 `fuzziness` 支援 |
| 前綴匹配 | 需特殊索引 | edge_ngram 原生支援 |

**結合使用的優勢**：PostgreSQL 作為唯一的資料來源 (Source of Truth)，保證資料完整性；Elasticsearch 作為搜尋引擎，提供高效的模糊查詢、全文檢索、拼寫容錯等功能。

## 資料流 (Dual-Write Pattern)

### 寫入流程（新增/更新/刪除）
```
Client Request
    │
    ▼
[1] 寫入 PostgreSQL (Source of Truth)
    │
    ├── 成功 ──▶ [2] 同步至 Elasticsearch (索引)
    │                  │
    │                  ├── 成功 ──▶ 回傳結果
    │                  └── 失敗 ──▶ 記錄警告，仍回傳成功 (最終一致性)
    │
    └── 失敗 ──▶ 回傳錯誤
```

### 搜尋流程
```
Client: GET /api/products/search?q=iphne
    │
    ▼
[1] 查詢 Elasticsearch (模糊匹配)
    │   回傳: [(uuid_1, score: 8.5), (uuid_2, score: 3.2)]
    │
    ▼
[2] 批次查詢 PostgreSQL (WHERE id = ANY($1))
    │   回傳完整商品資料
    │
    ▼
[3] 依照 ES 相關性分數排序後回傳
```

## API 端點

| 方法 | 路徑 | 說明 | 資料流 |
|------|------|------|--------|
| `GET` | `/health` | 健康檢查 | - |
| `POST` | `/api/products` | 新增商品 | PG → ES |
| `GET` | `/api/products/{id}` | 依 ID 查詢 | PG |
| `GET` | `/api/products/search?q=keyword` | 模糊搜尋 | ES → PG |
| `PUT` | `/api/products/{id}` | 更新商品 | PG → ES |
| `DELETE` | `/api/products/{id}` | 刪除商品 | PG → ES |
| `POST` | `/api/products/seed` | 植入範例資料 | PG → ES (批次) |

## Elasticsearch 索引設計

### 自定義分析器
- **autocomplete_analyzer**: 使用 `edge_ngram` filter (min=2, max=10)，支援前綴匹配和部分輸入搜尋
- **search_analyzer**: 標準分詞 + 小寫轉換，用於搜尋時的查詢分析

### 搜尋策略 (Bool Query)
1. **multi_match + fuzziness**: 跨 `name` 和 `description` 欄位模糊搜尋，容許拼寫錯誤
2. **match_phrase_prefix**: 支援前綴匹配 (輸入 "iph" 能找到 "iPhone")
3. **match with autocomplete**: 利用 edge_ngram 進行部分匹配

## 快速啟動

### 使用 Docker Compose（推薦）

```bash
cd pg_es_search_example
docker-compose up -d
```

等待所有服務就緒後（約 30-60 秒），即可使用 API。

### 本地開發

```bash
# 先啟動 PostgreSQL 和 Elasticsearch
docker-compose up -d postgres elasticsearch

# 使用本地 .env 設定
cd pg_es_search_example
cargo run
```

## 使用範例

### 1. 植入範例資料
```bash
curl -X POST http://localhost:8080/api/products/seed
```

### 2. 模糊搜尋（拼寫容錯）
```bash
# 搜尋 "iphne" (故意拼錯) 仍能找到 "iPhone 15 Pro Max"
curl "http://localhost:8080/api/products/search?q=iphne"

# 搜尋 "rust programming"
curl "http://localhost:8080/api/products/search?q=rust+programming"

# 前綴搜尋
curl "http://localhost:8080/api/products/search?q=sam"

# 中文搜尋
curl "http://localhost:8080/api/products/search?q=PostgreSQL"
```

### 3. 新增商品
```bash
curl -X POST http://localhost:8080/api/products \
  -H "Content-Type: application/json" \
  -d '{"name": "MacBook Pro 16", "description": "Apple M3 Max chip laptop", "category": "Electronics", "price": 2499.0}'
```

### 4. 查詢單一商品
```bash
curl http://localhost:8080/api/products/{id}
```

### 5. 更新商品
```bash
curl -X PUT http://localhost:8080/api/products/{id} \
  -H "Content-Type: application/json" \
  -d '{"price": 999.0}'
```

### 6. 刪除商品
```bash
curl -X DELETE http://localhost:8080/api/products/{id}
```

## 技術棧

| 元件 | 技術 | 版本 |
|------|------|------|
| Web 框架 | Actix-web | 4.x |
| PostgreSQL 客戶端 | sqlx | 0.8 |
| Elasticsearch 客戶端 | elasticsearch-rs | 8.15 |
| 序列化 | serde + serde_json | 1.x |
| 資料庫 | PostgreSQL | 16 |
| 搜尋引擎 | Elasticsearch | 8.17.0 |
| 容器編排 | Docker Compose | 3.8 |
