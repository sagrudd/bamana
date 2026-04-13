# Expected Outputs For `forensic_inspect`

Naming convention:

* `forensic_inspect.<fixture-id>.success.json`
* `forensic_inspect.<fixture-id>.failure.json`

Examples to reserve:

* `forensic_inspect.tiny.clean.bam.success.json`
* `forensic_inspect.tiny.forensic.bam.rg_pg_inconsistent.success.json`
* `forensic_inspect.tiny.forensic.bam.readname_shift.success.json`
* `forensic_inspect.tiny.forensic.bam.concatenated_signature.success.json`
* `forensic_inspect.tiny.invalid.bam.truncated_record.failure.json`

Semantic intent:

* clean fixtures reserve a no-anomaly provenance baseline
* forensic fixtures reserve parseable-but-suspicious success outputs
* invalid fixtures reserve parse-failure outputs so provenance anomalies remain
  distinct from malformed files
