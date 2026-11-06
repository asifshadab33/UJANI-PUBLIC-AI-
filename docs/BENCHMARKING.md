# UJANI reproducible benchmark protocol

## Purpose

Measure whether UJANI completes the same useful workload with acceptable latency, output quality, resource use and energy. Do not claim distributed efficiency from device count, theoretical FLOPS, or model-reported estimates.

## Required comparison

Every benchmark run records:

- workload ID and versioned input fixture hash;
- operation and task-graph schema version;
- exact model/runtime/adapter versions and model artifact digest where applicable;
- device OS, CPU architecture/core count, RAM and verified accelerator/runtime details;
- policy mode and whether execution was local CPU, local GPU, deterministic tool or external provider;
- warm/cold cache state and thermal/power conditions;
- wall-clock duration, peak memory, CPU utilisation, GPU utilisation/VRAM and failures/retries;
- output digest and task-specific quality score, reviewed independently of the model;
- energy value with provenance labelled **direct meter**, **OS estimate**, or **modelled estimate**.

## Fair comparison rules

1. Compare identical input fixtures and equivalent output requirements.
2. Record the same model and quality target when comparing execution placements.
3. Run at least five measured repetitions after one separately reported warm-up; publish median and p95, not only the fastest run.
4. Record failed and timed-out runs. Do not discard them from reliability statistics.
5. Measure network energy and retry cost for distributed runs where instrumentation permits.
6. Never wake a sleeping/off device solely to make a benchmark look favourable.
7. Do not combine direct measurements with estimated values in one total without clearly separating the categories.
8. Publish the hardware/software setup, raw anonymised measurements and analysis script with each benchmark report.

## Initial workload suite

- Deterministic text analysis and word count (no model).
- Short local chat prompt (same prompt, same model).
- Text summarisation at 8 KiB, 64 KiB and 256 KiB.
- PDF text extraction using a fixed text-based PDF fixture.
- Translation using a fixed language pair and independent quality rubric.
- GPU-required inference and CPU fallback as separate cases.

## Acceptance gates before public efficiency claims

- Functional equivalence: output passes the same acceptance rubric.
- Reliability: report success rate, timeout rate and p95 latency.
- Resource safety: no thermal-critical execution, memory overcommit or interactive-user disruption.
- Privacy: local-only runs verified with network disabled; external runs disclose exact destination and approved data scope.
- Energy: compare equivalent completed work, including network and retry overhead; publish direct/estimated provenance separately.

## Current status

This document defines the benchmark method. It does not mean UJANI currently has a complete cross-platform telemetry collector, automatic benchmark runner, independent quality evaluator, or direct energy meter. Those must be implemented and validated before performance or environmental claims are made.
