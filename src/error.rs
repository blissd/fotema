#[derive(Debug)]
pub enum Error {
    RepositoryError(String),
    FileSystemError(String),
    MetadataError(String),
    ScannerError(String),
}
