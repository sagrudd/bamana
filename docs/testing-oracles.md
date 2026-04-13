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
