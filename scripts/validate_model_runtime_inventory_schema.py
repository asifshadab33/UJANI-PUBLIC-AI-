#!/usr/bin/env python3
"""Validate the shared UJANI model/runtime inventory and task contracts."""
from __future__ import annotations

import copy
import hashlib
import json
import jsonschema
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "schemas/model-runtime-inventory-v1.schema.json"


EXPECTED_SHARED_ARTIFACTS = {
    "schemas/model-runtime-inventory-v1.schema.json": "8ff1a1dbbabd58636a3ef40e55b5c44168776522",
    "schemas/personal-enterprise-task-contract-v1.schema.json": "566a0d72babd58636a3ef40e55b5c44168776522",
    "schemas/test-vectors/consent-binding-v1.json": "dcd7021560212504810e8438e14efd591e5e657d",
    "schemas/test-vectors/model-runtime-inventory-v1.json": "543d645d0154c9263ff7784ba4e203f54e498c3c",
}


def _git_blob_sha1(path: Path) -> str:
    data = path.read_bytes()
    header = f"blob {len(data)}\\0".encode("ascii")
    return hashlib.sha1(header + data).hexdigest()


def validate_shared_contract_manifest() -> None:
    """Fail CI if either repository drifts from the shared v1 contract artifacts."""
    manifest_path = ROOT / "schemas/shared-contract-manifest-v1.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    assert manifest.get("manifest_version") == 1
    assert manifest.get("hash_algorithm") == "git_blob_sha1"
    assert manifest.get("artifacts") == EXPECTED_SHARED_ARTIFACTS
    for relative_path, expected_sha in EXPECTED_SHARED_ARTIFACTS.items():
        artifact = ROOT / relative_path
        actual_sha = _git_blob_sha1(artifact)
        assert actual_sha == expected_sha, (
            f"shared contract drift in {relative_path}: "
            f"expected {expected_sha}, got {actual_sha}"
        )


def main() -> None:
    validate_shared_contract_manifest()
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    assert schema["$schema"] == "https://json-schema.org/draft/2020-12/schema"
    assert schema["$id"] == "urn:ujani:model-runtime-inventory:v1"
    assert schema["type"] == "object" and schema["additionalProperties"] is False
    assert schema["properties"]["schema_version"]["const"] == 1
    assert set(schema["required"]) == {
        "schema_version", "generated_at_unix_ms", "platform", "hardware", "runtimes", "models"
    }
    device_ref = schema["properties"]["device_ref"]
    assert device_ref["type"] == ["string", "null"]
    assert device_ref["pattern"] == "^[A-Za-z0-9._:-]+$"
    assert schema["properties"]["runtimes"]["maxItems"] <= 16
    assert schema["properties"]["models"]["maxItems"] <= 512
    assert schema["properties"]["hardware"]["properties"]["accelerators"]["maxItems"] <= 32
    evidence = schema["$defs"]["evidence"]
    assert evidence["additionalProperties"] is False
    assert evidence["properties"]["confidence"]["enum"] == [
        "verified", "reported", "estimated", "unknown"
    ]
    fixture_path = SCHEMA_PATH.parent / "test-vectors/model-runtime-inventory-v1.json"
    fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
    validator = jsonschema.Draft202012Validator(schema)
    validator.validate(fixture)
    assert fixture["schema_version"] == 1
    assert fixture["models"][0]["estimated_min_memory_mb"] == 3_374

    # An empty report is valid but means "not reported", not "no capability".
    empty_inventory = copy.deepcopy(fixture)
    empty_inventory["models"] = []
    validator.validate(empty_inventory)

    # Unknown top-level and nested fields must not be silently accepted.
    unknown_top_level = copy.deepcopy(fixture)
    unknown_top_level["unexpected_field"] = True
    assert list(validator.iter_errors(unknown_top_level))

    unknown_model_field = copy.deepcopy(fixture)
    unknown_model_field["models"][0]["unrecognised_capability_claim"] = True
    assert list(validator.iter_errors(unknown_model_field))
    print("Model/runtime inventory v1 schema and shared fixture checks passed.")

    from validate_personal_enterprise_task_contract import main as validate_task_contract
    validate_task_contract()


if __name__ == "__main__":
    main()
