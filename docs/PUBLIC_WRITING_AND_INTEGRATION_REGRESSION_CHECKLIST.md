# Public AI writing shortcuts and integration directory — regression checklist

This checklist is for the controlled alpha review of issue #184. The directory contains external links only; it is not an account connector.

## Writing shortcuts
- [ ] Open the desktop app and confirm all six actions are visible: grammar, professional rewrite, email draft, summary, plain-language explanation and translation.
- [ ] Activate each action with a mouse and with keyboard Enter/Space.
- [ ] Confirm the selected template fills the shared prompt composer, scrolls to the task workspace with the local Ollama action below and moves keyboard focus to the composer.
- [ ] Confirm selecting a template does not submit a task, start inference, download a model or change execution policy. The user must explicitly press the real local Ollama action to start inference.
- [ ] Edit the prompt, confirm Ollama/model readiness, then explicitly run it once using the real local AI action (not the separate word-count demo); verify there is no duplicate submission.
- [ ] Confirm local-only policy remains selected and no cloud request is initiated by choosing a template.
- [ ] Confirm long text and non-Latin text can be pasted and edited normally.

## Integration directory
- [ ] Use the sidebar Integrations link; confirm it scrolls to the directory and receives focus when activated by keyboard.
- [ ] Confirm all six links are ordinary anchors and open only after explicit activation.
- [ ] Confirm each anchor has `target="_blank"` and `rel="noopener noreferrer"`.
- [ ] Confirm no link is opened automatically on first render.
- [ ] Confirm the disclosure says accounts are not connected and external providers have independent terms and privacy policies.
- [ ] Verify the directory remains readable at 320 px, 768 px and desktop widths without horizontal overflow.
- [ ] Verify all links point to the intended official provider domains before release.

## Failure and privacy checks
- [ ] Browser preview remains usable without native runtime services.
- [ ] A blocked or unavailable external website does not affect UJANI task execution.
- [ ] No credentials, tokens, prompts or files are passed to the directory.
- [ ] No network call is made when the directory is rendered.

Record OS, app commit, viewport, result and sanitized reproduction steps for every failure. Do not include prompts, document contents, credentials or personal file paths in bug reports.
