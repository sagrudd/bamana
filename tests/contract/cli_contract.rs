use super::{read_utf8, repo_root};

fn cli_source() -> String {
    read_utf8(&repo_root().join("src").join("cli.rs"))
}

#[test]
fn cli_source_declares_stable_global_option_and_commands() {
    let source = cli_source();

    for token in [
        "pub json_pretty: bool",
        "Benchmark(BenchmarkArgs)",
        "Identify(IdentifyArgs)",
        "Enumerate(EnumerateArgs)",
        "Subsample(SubsampleArgs)",
        "InspectDuplication(InspectDuplicationArgs)",
        "Deduplicate(DeduplicateArgs)",
        "ForensicInspect(ForensicInspectArgs)",
        "Consume(ConsumeArgs)",
        "Checksum(ChecksumArgs)",
        "Merge(MergeArgs)",
        "Sort(SortArgs)",
        "Verify(BamPathArgs)",
        "CheckEof(BamPathArgs)",
        "Header(BamPathArgs)",
        "CheckMap(CheckMapArgs)",
        "CheckIndex(CheckIndexArgs)",
        "Index(IndexArgs)",
        "Summary(SummaryArgs)",
        "Validate(ValidateArgs)",
        "CheckTag(CheckTagArgs)",
        "CheckSort(CheckSortArgs)",
    ] {
        assert!(
            source.contains(token),
            "missing CLI contract token {token} in src/cli.rs"
        );
    }
}

#[test]
fn cli_source_declares_key_subcommand_flags() {
    let source = cli_source();

    for token in [
        "long = \"bam\"",
        "long = \"input\"",
        "long = \"profile\"",
        "long = \"fastq\"",
        "long = \"fraction\"",
        "long = \"identity\"",
        "long = \"out\"",
        "long = \"report\"",
        "long = \"seed\"",
        "long = \"keep\"",
        "long = \"mode\"",
        "long = \"create-index\"",
        "long = \"mapped-only\"",
        "long = \"primary-only\"",
        "long = \"recursive\"",
        "long = \"dry-run\"",
        "long = \"order\"",
        "long = \"queryname-suborder\"",
        "long = \"memory-limit\"",
        "long = \"create-index\"",
        "long = \"verify-checksum\"",
        "long = \"sample\"",
        "long = \"read-group\"",
        "long = \"platform\"",
        "long = \"min-block-size\"",
        "long = \"sample-records\"",
        "long = \"full-scan\"",
        "long = \"max-findings\"",
        "long = \"inspect-header\"",
        "long = \"inspect-rg\"",
        "long = \"inspect-pg\"",
        "long = \"inspect-readnames\"",
        "long = \"inspect-tags\"",
        "long = \"inspect-duplication\"",
        "long = \"emit-removed-report\"",
        "long = \"reindex\"",
        "long = \"force\"",
        "long = \"include-glob\"",
        "long = \"exclude-glob\"",
        "long = \"prefer-index\"",
        "long = \"prefer-csi\"",
        "long = \"include-mapq-hist\"",
        "long = \"include-flags\"",
        "long = \"require-type\"",
        "long = \"count-hits\"",
        "long = \"max-errors\"",
        "long = \"header-only\"",
    ] {
        assert!(
            source.contains(token),
            "missing CLI flag contract token {token} in src/cli.rs"
        );
    }
}
