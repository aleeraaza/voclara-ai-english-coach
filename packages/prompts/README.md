# Prompts

Every prompt file has front matter with a `version`. The API stores that version
on each session (`sessions.prompt_version`) and each score (`scores.grader_version`),
so a change in grading can always be traced to a prompt change.

Never edit a published version in place: copy `v1.md` to `v2.md`, change it,
and switch the API config to the new version.
