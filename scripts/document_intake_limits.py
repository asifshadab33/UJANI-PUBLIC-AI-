"""Conservative preflight limits for local document/media ingestion."""
from dataclasses import dataclass

@dataclass(frozen=True)
class DocumentMetrics:
    size_bytes: int
    page_count: int
    extracted_characters: int
    embedded_objects: int
    encrypted: bool

def intake_denials(metrics: DocumentMetrics, *, max_bytes: int = 50_000_000,
                   max_pages: int = 500, max_characters: int = 5_000_000,
                   max_embedded_objects: int = 2_000) -> list[str]:
    denials = []
    if metrics.size_bytes < 0 or metrics.size_bytes > max_bytes: denials.append("file_size_limit")
    if metrics.page_count < 0 or metrics.page_count > max_pages: denials.append("page_count_limit")
    if metrics.extracted_characters < 0 or metrics.extracted_characters > max_characters: denials.append("text_limit")
    if metrics.embedded_objects < 0 or metrics.embedded_objects > max_embedded_objects: denials.append("embedded_object_limit")
    if metrics.encrypted: denials.append("encrypted_document_requires_user_action")
    return denials
