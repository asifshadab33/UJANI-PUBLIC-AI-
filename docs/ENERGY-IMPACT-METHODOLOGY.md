# UJANI energy impact measurement — methodology and target

## Why this exists

UJANI's environmental objective must be measured against equivalent completed work, not inferred from the number of computers enrolled or the amount of compute moved away from a data centre.

The International Energy Agency's 2026 analysis estimates global data-centre electricity use at about 485 TWh in 2025 and projects around 950 TWh in 2030 in its central outlook. Those are scenario estimates, not a guaranteed future. UJANI will use published baselines as context, not as proof of its own impact.

## What 0.001% would mean

As a scale illustration only, 0.001% of 950 TWh is 0.0095 TWh, or **9.5 GWh per year**. That is equivalent to an average continuous power level of approximately 1.08 MW.

This is an ambitious long-term reference point, not a UJANI forecast. It does not mean UJANI can currently save that energy, and it must never be presented as an achieved result.

## Required comparison boundary

For every benchmark, compare the same useful workload and output-quality target.

**Central baseline**
- Measure or estimate facility electricity attributable to the workload, including IT and facility overhead on a consistent basis.
- Record model, hardware, batch size, input/output sizes, duration, quality checks and retry count.

**Distributed alternative**
- Measure incremental device electricity above the device's normal idle/owner-use baseline.
- Include network transfer energy, additional cooling/facility energy where attributable, failed attempts, retries and checkpoint overhead.
- Record whether the device would otherwise have been asleep, idle, or doing useful foreground work.
- Do not count an already-running PC's entire power draw as incremental if it would have been on anyway; do not assume its incremental energy is zero without measurement.

**Result**
- Net estimated energy saving = central baseline energy − (incremental distributed device energy + network energy + additional cooling + retry/failure energy).
- A negative result is valid and must be shown: distributed execution sometimes consumes more energy.
- A positive result is a workload-level estimate. It does not prove that a data centre shut down equipment or reduced its metered electricity consumption.

The core helper `compare_energy_scenarios` enforces an explicit equivalent-workload verification flag, rejects a missing baseline and includes retry/network/cooling overhead. It does not read sensors or invent measurements.

## Environmental claims

Do not convert energy into CO2e unless the electricity carbon intensity for each location and time period is supported by a defensible source. For geographically distributed work, use location-specific electricity factors and state their vintage and uncertainty.

Do not claim water savings, avoided data-centre construction, or embodied-carbon savings from energy accounting alone. Those require separate evidence. Device manufacturing, replacement, battery wear and hardware lifetime also matter.

## Initial pilot measurement plan

1. Choose a small, repeatable, non-sensitive CPU workload with deterministic output.
2. Run it on one baseline server and one enrolled test workstation.
3. Verify identical output and workload size.
4. Measure wall energy with external power meters where possible; software estimates are secondary and must be labelled.
5. Measure network bytes and estimate network energy using a documented factor with uncertainty bounds.
6. Repeat at least 30 times across idle, normal-use and deliberately interrupted conditions.
7. Include failed runs and retries in the totals.
8. Publish the method, raw anonymised measurements, hardware, software versions and uncertainty.
9. Compare the total energy per successful, quality-checked result.
10. Do not generalise a pilot result to global data-centre reductions.

## Success measures

- Wh per successful, quality-checked workload.
- Incremental device Wh and network Wh.
- Retry/failure energy share.
- Completion time and availability.
- Owner interruption rate and user opt-out rate.
- Data-centre baseline Wh versus distributed alternative Wh.
- Energy savings per workload and the share of workloads where distributed execution is actually lower-energy.

UJANI's scheduler should be allowed to leave a device unused or select a central service whenever that is safer, faster or lower-energy.
