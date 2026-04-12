# Compatibility Policy

## General Rule

Bamana should preserve backward compatibility for documented CLI and JSON
contracts within a contract baseline unless an explicit reviewed change is made.

## Consumer Expectations

Automation consumers should be able to rely on:

* literal command names
* documented option names
* stable JSON field names
* stable enum strings
* stable error-code naming patterns

## Evolving Incomplete Commands

Some commands in the repository have honest partial implementations or planned
schemas. They may evolve in depth, but not by silently changing what existing
fields mean.

Examples:

* bounded scan outputs must stay bounded in meaning
* shallow verification must stay shallow in meaning
* checksum modes must stay explicitly defined
* sort/merge content preservation must stay opt-in and explicit

## Planned Versus Implemented Commands

The spec layer may describe commands such as `explode` before the implementation
lands. These planned contracts must be marked clearly in docs and examples so
consumers do not assume runtime availability.
