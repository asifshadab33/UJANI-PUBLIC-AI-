"""Fail-closed policy for sending a task to an external AI service."""
from dataclasses import dataclass

@dataclass(frozen=True)
class ExternalAssistanceRequest:
    user_enabled_external_assistance: bool
    task_approved_for_external_assistance: bool
    payload_minimized: bool
    contains_local_file_content: bool
    file_content_explicitly_selected: bool
    destination_configured: bool

def external_assistance_denials(request: ExternalAssistanceRequest) -> list[str]:
    denials = []
    if not request.user_enabled_external_assistance:
        denials.append("external_assistance_disabled")
    if not request.task_approved_for_external_assistance:
        denials.append("task_not_approved")
    if not request.payload_minimized:
        denials.append("payload_not_minimized")
    if request.contains_local_file_content and not request.file_content_explicitly_selected:
        denials.append("file_content_not_explicitly_selected")
    if not request.destination_configured:
        denials.append("destination_not_configured")
    return denials

def may_use_external_assistance(request: ExternalAssistanceRequest) -> bool:
    return not external_assistance_denials(request)
