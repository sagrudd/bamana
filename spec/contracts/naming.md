# Naming Rules

## JSON Fields

* Use `snake_case`.
* Avoid abbreviations unless they are domain-standard, such as `mapq`.
* Prefer explicit booleans such as `full_file_scanned` over ambiguous names.

## Command Names

* Use stable literal CLI names.
* Multiword commands use snake_case in CLI names where already established:
  `check_eof`, `check_sort`, `check_map`, `check_index`, `check_tag`.

## Enum Strings

* Use lowercase or kebab-case consistently according to the command domain.
* Do not change enum strings casually; they are public contract values.

## Error Codes

* Use lowercase snake_case.
* Error codes are machine-readable public identifiers.
* Prefer explicit names such as `checksum_mismatch` or `incompatible_headers`
  over vague names.

## Note And Message Wording

* Keep `message` concise and human-readable.
* Keep `detail` specific.
* Keep `hint` actionable.
* Avoid overstating guarantees in notes, messages, or hints.
