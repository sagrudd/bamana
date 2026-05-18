# Testing And Oracle Policy

External parser crates such as `noodles` may still be useful in the Bamana test
strategy even after they are demoted from hot-path production roles.

## Valid Oracle Uses

Examples of acceptable oracle uses:

* compare Bamana-native header parsing against an independent parser
* compare BAM serialization behavior in small fixtures
* validate edge-case fixture behavior in differential tests
* cross-check compatibility of small synthetic examples

## Invalid Oracle Uses

Oracle usage must not become a hidden production dependency. In particular:

* tests must not justify moving `noodles` into hot paths
* convenience in tests must not drive production architecture
* compatibility checks must remain separate from the core execution engine

## Review Rule

If `noodles` or similar crates are added to tests:

* the test should state that the crate is used as an oracle or compatibility
  comparator
* the same crate should not quietly become a production-path requirement for
  the command being tested

## Native Header Oracle Boundary

`tests/header_oracle.rs` is the explicit test-only oracle surface for Milestone
2 header comparisons. It may use `noodles` to compare valid header fixtures or
to document compatibility differences, but production header and verify paths
must remain Bamana-native and free of direct `noodles` imports.

Malformed-header failure expectations must stay in native unit tests and must
not depend on an external parser. Differential tests may explain where another
parser is stricter or more permissive, but the expected Bamana behavior remains
owned by the native header codec.

## Native Scanner Oracle Boundary

Milestone 3 scanner expectations follow the same boundary.
Malformed-record failure expectations must stay in Bamana-native scanner and record-view tests;
they must not depend on `noodles` or another external parser to define the
expected error. Differential scanner checks may compare native scanner views
against Bamana's owned `RecordLayout` bridge or a clearly labelled test-only
oracle, but production scanner and migrated record hot paths must remain free of
direct `noodles` imports.
