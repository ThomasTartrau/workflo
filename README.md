<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/logo/workflo-logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/logo/workflo-logo-light.svg">
  <img alt="workflo" src="assets/logo/workflo-banner.png" width="560">
</picture>

[![pipeline status](https://img.shields.io/gitlab/pipeline-status/ThomasTartrau%2Fworkflo?branch=main&style=for-the-badge&logo=gitlab&logoColor=white)](https://gitlab.com/ThomasTartrau/workflo/-/pipelines)
[![crates.io](https://img.shields.io/crates/v/workflo.svg?style=for-the-badge&logo=rust&logoColor=white)](https://crates.io/crates/workflo)
[![docs.rs](https://img.shields.io/docsrs/workflo?style=for-the-badge&logo=docs.rs)](https://docs.rs/workflo)
![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)
![MSRV](https://img.shields.io/badge/MSRV-1.75-orange?style=for-the-badge&logo=rust&logoColor=white)

**Durable workflows on the Postgres you already run.**

</div>

workflo is a workflow engine for Rust backed by PostgreSQL. Every execution is a state machine
persisted in the database, and workers run its operations with retries, batches and dependencies
between executions. No broker, no extra service: a `PgPool` is all it needs.

- Describe steps as `Operation`s and map workflows to them with a `WorkflowResolver`.
- Run one or more `Worker`s on top of `PgExecutionStore`.
- Create the tables with `workflo::migrate(&pool).await?`.

## Examples

- [`examples/simple_workflow.rs`](examples/simple_workflow.rs): a single-step notification workflow.
- [`examples/order_processing`](examples/order_processing): an order workflow in three steps
  (reserve stock, charge payment, ship) with its own operations, errors and resolver.

## License

MIT
