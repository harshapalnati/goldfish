# Benchmark Suites

This folder contains retrieval benchmark assets for Goldfish.
It follows an RTEB-style retrieval setup (Hugging Face RTEB blog, Oct 1, 2025),
where ranked retrieval quality is measured primarily with `nDCG@10`.

## Metrics

- `Recall@1`
- `Recall@3`
- `Recall@5`
- `MRR` (Mean Reciprocal Rank)
- `nDCG@k` (default `k=10`)
- Latency (`avg` and `p95`)

## Dataset format (JSONL)

### `datasets/sample_memories.jsonl`

Each line:

```json
{"id":"m1","content":"User prefers concise answers","memory_type":"preference","importance":0.9}
```

### `datasets/sample_queries.jsonl`

Each line:

```json
{"query_id":"q1","query":"concise user preference","relevant_ids":["m1"],"relevance":{"m1":3}}
```

`relevance` is optional. If omitted, `relevant_ids` are treated as binary relevance (`1`).

## Run harness

```bash
cargo run --example benchmark_suite
```

LanceDB:

```bash
cargo run --example benchmark_suite --features lancedb -- --vector-backend lancedb
```

Reports are written to `benchmark_suites/results/`.

## Publishable PoC Harness

For a larger deterministic benchmark and Markdown report export:

```bash
cargo run --example benchmark_poc -- --name poc_publish --vector-backend file
cargo run --example benchmark_poc --features lancedb -- --name poc_publish --vector-backend lancedb
```

See `benchmark_suites/PUBLISHING_POC.md` for publishing workflow.
