# Bamana Global Options

## `--json-pretty`

Purpose:
Requests pretty-printed JSON output.

Contract:

* Present on the top-level CLI and accepted for every subcommand.
* Changes JSON formatting only.
* Must not change command semantics, field presence, or exit-code policy.

## Reserved Global Surface

The current repository does not expose additional global flags, but the project
expects future growth in areas such as:

* quiet/verbose control
* contract/schema version reporting
* diagnostic tracing controls

Any future global option should be added conservatively because global CLI
surface changes are contract changes for automation.
