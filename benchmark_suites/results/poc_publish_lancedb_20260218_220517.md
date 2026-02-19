# Goldfish Benchmark Report

- Generated: `2026-02-18T22:05:17.081825600+00:00`
- Backend: `lancedb`
- Suite: `Goldfish PoC Retrieval Benchmark`

## Dataset

- Topics: 8
- Memories: 2400
- Queries: 240
- Ingestion time: 5490602.86 ms

## Aggregate Metrics

| Recall@1 | Recall@3 | Recall@5 | MRR | nDCG | Avg Latency (ms) | P95 Latency (ms) |
|---:|---:|---:|---:|---:|---:|---:|
| 0.0342 | 0.0933 | 0.1408 | 0.8785 | 0.6025 | 513.3448 | 533.2154 |

## Per Run

| Run | Recall@1 | Recall@3 | Recall@5 | MRR | nDCG | Avg Latency (ms) | P95 Latency (ms) |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 0.0342 | 0.0933 | 0.1408 | 0.8785 | 0.6020 | 512.3005 | 525.9405 |
| 2 | 0.0342 | 0.0933 | 0.1408 | 0.8785 | 0.6028 | 511.3593 | 525.9557 |
| 3 | 0.0342 | 0.0933 | 0.1408 | 0.8785 | 0.6028 | 516.3746 | 547.7501 |

## Repro Command

```bash
target\debug\examples\benchmark_poc.exe --name poc_publish --vector-backend lancedb
```
