# Goldfish Benchmark Report

- Generated: `2026-02-19T01:29:13.105484400+00:00`
- Backend: `file`
- Suite: `Goldfish PoC Retrieval Benchmark`

## Dataset

- Topics: 8
- Memories: 2400
- Queries: 240
- Ingestion time: 8922.61 ms

## Profiles

| Profile | w_text | w_importance | w_vector | Recall@1 | Recall@3 | Recall@5 | MRR | nDCG | Avg Latency (ms) | P95 Latency (ms) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| baseline | 0.250 | 0.250 | 0.500 | 0.0291 | 0.0845 | 0.1326 | 0.7685 | 0.5180 | 410.4339 | 430.0389 |
| tuned | 0.380 | 0.120 | 0.500 | 0.0253 | 0.0728 | 0.1150 | 0.6924 | 0.4329 | 410.7780 | 429.2555 |

## Sweep Delta

Compared `baseline` -> `tuned`

| Metric | Delta |
|---|---:|
| Recall@1 | -0.0038 |
| Recall@3 | -0.0117 |
| Recall@5 | -0.0176 |
| MRR | -0.0762 |
| nDCG | -0.0851 |
| Avg Latency (ms) | 0.3441 |
| P95 Latency (ms) | -0.7834 |

## Repro Command

```bash
target\debug\examples\benchmark_poc.exe --name poc_publish_file --vector-backend file --sweep --export-dataset --reset-data --data-dir ./benchmark_cortex_data/poc_file
```
