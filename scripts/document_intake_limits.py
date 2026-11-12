"""Conservative preflight bounds for local document/media ingestion."""

from dataclasses import dataclass


@dataclass(frozen=True)
class DocumentMetrics:
    size_bytes: int
    page_count: int
    extracted_characters: int
    embedded_objects: int
    encrypted: bool


def intake_denials(
    metrics: DocumentMetrics,
    *,
    max_bytes: int = 50_000_000,
    max_pages: int = 500,
    max_characters: int = 5_000_000,
    max_embedded_objects: int = 2_000,
) -> list[str]:
    if not isinstance(metrics, DocumentMetrics):
        return ["invalid_metrics"]

    denials: list[str] = []
    metric_values = {
        "file_size_limit": metrics.size_bytes,
        "page_count_limit": metrics.page_count,
        "text_limit": metrics.extracted_characters,
        "embedded_object_limit": metrics.embedded_objects,
    }
    for reason, value in metric_values.items():
        if type(value) is not int or value < 0:
            denials.append("invalid_metric_value")
    limits = (max_bytes, max_pages, max_characters, max_embedded_objects)
    if any(type(limit) is not int or limit <= 0 for limit in limits):
        denials.append("invalid_limit_configuration")

    if type(metrics.encrypted) is not bool:
        denials.append("invalid_encryption_state")
    elif metrics.encrypted:
        denials.append("encrypted_document_requires_user_action")

    if not any(reason in denials for reason in (
        "invalid_metric_value", "invalid_limit_configuration"
    )):
        for (reason, value), limit in zip(metric_values.items(), limits):
            if value > limit:
                denials.append(reason)
    return list(dict.fromkeys(denials))


def may_process_document(metrics: DocumentMetrics, **limits: int) -> bool:
    return not intake_denials(metrics, **limits)
