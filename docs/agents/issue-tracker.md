# Issue tracker: GitHub

Issues and specs live in https://github.com/HoldEvan/Easy-MD/issues.

## Operations

Use the workspace's deck tools when available. Call deck_context first
to verify the workspace and backend; follow its reported tool conventions.

When using gh directly, explicitly target --repo HoldEvan/Easy-MD
so this fork's work is not sent to upstream.

Use native GitHub sub-issues for parent/child relationships and native
issue dependencies for blocking relationships. If unavailable, record
Part of #<parent> and Blocked by: #<number> in the issue body.

When publishing a spec or ticket, create a GitHub issue.
When fetching a ticket, read its body, relevant comments and relationships.
When closing a ticket, record the outcome before closing it.

## Pull requests as a triage surface

**PRs as a request surface: no.**

## Wayfinding

Maps use wayfinder:map. Child tickets use the applicable type:
wayfinder:research, wayfinder:prototype, wayfinder:grilling or wayfinder:task.

Link child tickets to their map and record dependencies.
Choose unassigned, open tickets whose blockers are resolved.
Apply labels according to the invoked skill.
