# Invalid BAM Fixtures

This directory is reserved for deliberately malformed BAMs that exercise:

* missing EOF marker handling
* truncated alignment-record handling
* malformed auxiliary-field handling
* header/text mismatch handling

Preferred naming:

* `tiny.invalid.no_eof.bam`
* `tiny.invalid.truncated_record.bam`
* `tiny.invalid.bad_aux.bam`
* `tiny.invalid.header_mismatch.bam`
