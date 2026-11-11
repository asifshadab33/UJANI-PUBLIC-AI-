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
    if not isinstance(request, ExternalAssistanceRequest):
        return ["invalid_request"]

    denials: list[str] = []
    required_true = (
        ("user_enabled_external_assistance", request.user_enabled_external_assistance, "external_assistance_disabled"),
        ("task_approved_for_external_assistance", request.task_approved_for_external_assistance, "task_not_approved"),
        ("payload_minimized", request.payload_minimized, "payload_not_minimized"),
        ("destination_configured", request.destination_configured, "destination_not_configured"),
    )
    for field, value, denial in required_true:
        if value is not True:
            denials.append(denial if value is False else f"invalid_{field}")

    if request.contains_local_file_content is True:
        if request.file_content_explicitly_selected is not True:
            denials.append("file_content_not_explicitly_selected")
    elif request.contains_local_file_content is not False:
        denials.append("invalid_contains_local_file_content")

    if request.file_content_explicitly_selected not in (True, False) or type(request.file_content_explicitly_selected) is not bool:
        denials.append("invalid_file_content_selection")

    return list(dict.fromkeys(denials))

def may_use_external_assistance(request: ExternalAssistanceRequest) -> bool:
    return not external_assistance_denials(request)
