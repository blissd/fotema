pub enum Error {
    DatabaseError(String),
    FileSystemError(String),
    MetadataError(String),
    ScannerError(String),
}
