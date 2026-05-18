Native BAM Header Codec
=======================

Milestone 2 owns BAM header parsing and serialization above the native BGZF
substrate.

Header Model
------------

The native header model preserves both the declared SAM-style text and the
binary reference dictionary.

``raw_header_text``
   The SAM-style header text exactly as declared by the BAM header prefix.

``references``
   The binary reference dictionary in encounter order. This is authoritative for
   BAM decoding.

``reference_diagnostics``
   Non-fatal warnings when textual ``@SQ`` records disagree with the binary
   dictionary.

Header Command
--------------

``bamana header`` uses the native BGZF stream reader and native BAM header
codec. A successful response proves that the container and header prefix,
textual header, and binary reference dictionary were readable enough to parse.
It does not prove that alignment records, EOF state, or the complete BAM body
are valid.

Verify Command
--------------

``bamana verify`` uses the same native BGZF stream and BAM header codec for
header-level verification. It confirms BGZF container recognition, BAM magic,
and native header/reference-dictionary parsing. It does not scan alignment
records and does not report EOF-marker status.

Serialization
-------------

Native BAM header serialization is deterministic:

* BAM magic bytes are emitted first.
* ``l_text`` is the byte length of the selected SAM-style header text.
* Header text is emitted exactly as supplied by the native header view or
  command-specific mutation helper.
* ``n_ref`` is the binary reference count.
* Binary references are emitted in encounter-order index order.
* Each binary reference name includes exactly one required NUL terminator.
* Counts and lengths are checked before bytes are emitted.

Checksum code uses the shared native header checksum-domain serializer, so it
does not duplicate header representation rules.
