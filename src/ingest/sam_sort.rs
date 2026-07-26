use std::{
    io::BufRead,
    path::{Path, PathBuf},
};

use crate::{
    bam::{
        header::{rewrite_header_for_sort, serialize_bam_header_payload},
        sort::{QuerynameSubOrder, SortOrder},
        stream_sort::{StreamSortOptions, sort_record_stream},
    },
    error::AppError,
    ingest::{consume::ConsumeSortOrder, sam::SamRecordStream},
};

#[derive(Debug)]
pub struct StreamingSamSortOptions {
    pub output_path: PathBuf,
    pub force: bool,
    pub order: ConsumeSortOrder,
    pub threads: usize,
    pub memory_limit: u64,
    pub compression_level: u32,
}

#[derive(Debug)]
pub struct StreamingSamSortExecution {
    pub records_written: u64,
    pub run_count: usize,
    pub overwritten: bool,
    pub header_strategy: &'static str,
    pub reference_compatibility: &'static str,
    pub notes: Vec<String>,
}

pub fn execute_streaming_sam_sort<R: BufRead>(
    reader: R,
    input_label: &Path,
    options: &StreamingSamSortOptions,
) -> Result<StreamingSamSortExecution, AppError> {
    let (order, sort_order, sub_sort_order, queryname_suborder) = match options.order {
        ConsumeSortOrder::Coordinate => (SortOrder::Coordinate, "coordinate", None, None),
        ConsumeSortOrder::Queryname => (
            SortOrder::Queryname,
            "queryname",
            Some("queryname:lexicographical"),
            Some(QuerynameSubOrder::Lexicographical),
        ),
        ConsumeSortOrder::None => {
            return Err(AppError::InvalidConsumeRequest {
                path: options.output_path.clone(),
                detail: "Direct SAM stream sorting requires --sort coordinate or --sort queryname."
                    .to_string(),
            });
        }
    };

    let stream = SamRecordStream::new(reader, input_label)?;
    let header_text = rewrite_header_for_sort(stream.raw_header_text(), sort_order, sub_sort_order);
    let header_payload =
        serialize_bam_header_payload(&options.output_path, &header_text, stream.references())?;
    let result = sort_record_stream(
        &StreamSortOptions {
            output_path: options.output_path.clone(),
            force: options.force,
            order,
            queryname_suborder,
            threads: options.threads,
            memory_limit: options.memory_limit,
            compression_level: options.compression_level,
        },
        &header_payload,
        stream,
    )?;

    Ok(StreamingSamSortExecution {
        records_written: result.records_written,
        run_count: result.run_count,
        overwritten: result.overwritten,
        header_strategy: "streamed_sam_header",
        reference_compatibility: "compatible",
        notes: vec![
            "Uncompressed SAM was parsed directly from stdin without staging a complete SAM or unsorted BAM.".to_string(),
            format!(
                "External sorting retained at most the configured {}-byte record budget before spilling deterministic runs.",
                options.memory_limit
            ),
            format!(
                "Run sorting and ordered BGZF compression used up to {} worker thread(s) at compression level {}; the final stable multiway merge remained deterministic.",
                options.threads.max(1),
                options.compression_level
            ),
        ],
    })
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Cursor, path::Path};

    use crate::{
        bam::scan::BamScanner,
        ingest::{
            consume::ConsumeSortOrder,
            sam_sort::{StreamingSamSortOptions, execute_streaming_sam_sort},
        },
    };

    #[test]
    fn streams_sam_into_multiple_queryname_sorted_runs() {
        let output =
            std::env::temp_dir().join(format!("bamana-stream-sam-sort-{}.bam", std::process::id()));
        let sam = concat!(
            "@HD\tVN:1.6\tSO:unsorted\n",
            "@SQ\tSN:chr1\tLN:100\n",
            "z\t0\tchr1\t2\t60\t2M\t*\t0\t0\tAC\t!!\n",
            "a\t0\tchr1\t1\t60\t2M\t*\t0\t0\tGT\t!!\n",
        );
        let execution = execute_streaming_sam_sort(
            Cursor::new(sam),
            Path::new("<stdin>"),
            &StreamingSamSortOptions {
                output_path: output.clone(),
                force: true,
                order: ConsumeSortOrder::Queryname,
                threads: 2,
                memory_limit: 1,
                compression_level: 1,
            },
        )
        .expect("stream sort should succeed");

        assert_eq!(execution.records_written, 2);
        assert_eq!(execution.run_count, 2);
        let mut scanner = BamScanner::open(&output).expect("output should scan");
        assert!(
            scanner
                .header()
                .header
                .raw_header_text
                .contains("SO:queryname")
        );
        assert!(
            scanner
                .header()
                .header
                .raw_header_text
                .contains("SS:queryname:lexicographical")
        );
        let first = scanner
            .next_record()
            .expect("record should parse")
            .expect("first record should exist")
            .read_name()
            .to_string();
        let second = scanner
            .next_record()
            .expect("record should parse")
            .expect("second record should exist")
            .read_name()
            .to_string();
        assert_eq!(first, "a");
        assert_eq!(second, "z");
        assert!(scanner.next_record().expect("end should parse").is_none());
        fs::remove_file(output).expect("output should remove");
    }
}
