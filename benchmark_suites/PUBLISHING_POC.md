# Publishing PoC Benchmark

Use this workflow to generate benchmark artifacts you can commit to your repo.

## 1) Run file backend benchmark

```bash
cargo run --example benchmark_poc -- --name poc_publish --vector-backend file
```

## 2) Run LanceDB benchmark

```bash
cargo run --example benchmark_poc --features lancedb -- --name poc_publish --vector-backend lancedb
```

## 3) Publish artifacts

Commit files from:

- `benchmark_suites/results/*.json`
- `benchmark_suites/results/*.md`
- `benchmark_suites/datasets/generated/*.jsonl`

Each report includes:

- dataset size
- retrieval quality metrics (`Recall@1/3/5`, `MRR`, `nDCG`)
- latency metrics (`avg`, `p95`)
- exact reproduce command

## Notes

- This suite uses a deterministic synthetic dataset for reproducibility.
- For stronger external validity, add real-world query relevance sets using the same JSONL schema.
