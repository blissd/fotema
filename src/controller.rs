use crate::repo;
use crate::scanner;
use crate::Result;

pub struct Controller {
    repo: repo::Repository,
    scan: scanner::Scanner,
}

/// Summary of a scan
pub struct ScanSummary {
    /// Count of pictures scanned
    success_count: u32,
    // Count of pictures that could not be processed
    error_count: u32,
}

impl Controller {
    pub fn new(repo: repo::Repository, scan: scanner::Scanner) -> Controller {
        Controller { repo, scan }
    }

    pub fn scan(&self) -> Result<ScanSummary> {
        let mut summary = ScanSummary {
            success_count: 0,
            error_count: 0,
        };

        self.scan.visit_all(|pic| {
            let exif_date_time = pic.exif.and_then(|x| x.created_at);
            let fs_date_time = pic.fs.and_then(|x| x.created_at);
            let order_by_ts = exif_date_time.map(|d| d.to_utc()).or(fs_date_time);

            let pic = repo::Picture {
                path: pic.path,
                order_by_ts,
            };
            match self.repo.add(&pic) {
                Ok(_) => summary.success_count += 1,
                Err(_) => summary.error_count += 1,
            }
        });

        Ok(summary)
    }

    pub fn all(&self) -> Result<Vec<repo::Picture>> {
        self.repo.all()
    }
}
