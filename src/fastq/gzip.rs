use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    thread,
};

use flate2::read::MultiGzDecoder;

use crate::error::AppError;

pub(crate) fn open_maybe_gzip_reader(
    path: &Path,
    label: &Path,
) -> Result<Box<dyn BufRead>, AppError> {
    let file = File::open(path).map_err(|error| AppError::from_io(label, error))?;
    if is_gzip_fastq_path(path) {
        Ok(Box::new(BufReader::new(MultiGzDecoder::new(file))))
    } else {
        Ok(Box::new(BufReader::new(file)))
    }
}

pub(crate) fn is_gzip_fastq_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("gz"))
}

pub fn resolved_threads(requested_threads: usize) -> usize {
    let available = thread::available_parallelism()
        .map(|parallelism| parallelism.get())
        .unwrap_or(1);
    if requested_threads == 0 {
        available.max(1)
    } else {
        requested_threads.min(available).max(1)
    }
}
