# Expected Fixture Outputs

This tree is reserved for expected command outputs produced from real fixture
files.

Naming convention:

* success-path outputs:
  `expected/<command>/<fixture-id>.success.json`
* failure-path outputs:
  `expected/<command>/<fixture-id>.failure.json`

Where a fixture has multiple meaningful success variants, add a stable suffix:

* `expected/checksum/tiny.transforms.source.canonical.success.json`
* `expected/sort/tiny.transforms.source.coordinate.success.json`

Expected outputs should reflect the governed contract, not transient
implementation accidents.
