# Public AI release evidence gate (Batch K)

A release candidate remains blocked until the evidence set includes supported-OS
native builds, installer smoke tests, Ollama transport security, filesystem
boundary tests, consent/transport tests, document parser limits and fuzzing,
freeze/crash regressions, and an independent external test report.

The checklist validates evidence labels only; it does not verify the quality or
authenticity of attached reports. Release owners must attach immutable CI run
links, device/OS details, test versions and signed external findings. Passing
repository CI alone is not an external acceptance report.
