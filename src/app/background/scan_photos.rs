use relm4::prelude::*;
use relm4::Worker;

#[derive(Debug)]
pub enum ScanPhotosInput {
    ScanAll,
}

#[derive(Debug)]
pub enum ScanPhotosOutput {
    ScanAllCompleted(Vec<photos_core::scanner::Picture>),
}

pub struct ScanPhotos {
    scanner: photos_core::Scanner,
}

impl Worker for ScanPhotos {
    type Init = photos_core::Scanner;
    type Input = ScanPhotosInput;
    type Output = ScanPhotosOutput;

    fn init(scanner: Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { scanner }
    }

    fn update(&mut self, msg: ScanPhotosInput, sender: ComponentSender<Self>) {
        match msg {
            ScanPhotosInput::ScanAll => {
                println!("ScanAll");
                let start_at = std::time::SystemTime::now();
                let result = self.scanner.scan_all();
                let end_at = std::time::SystemTime::now();

                if let Ok(pics) = result {
                    let duration = end_at.duration_since(start_at).unwrap_or(std::time::Duration::new(0, 0));
                    println!("Scanned {} items in {} seconds", pics.len(), duration.as_secs());
                    let _ = sender.output(ScanPhotosOutput::ScanAllCompleted(pics));
                } else {
                    println!("Failed scanning: {:?}", result);
                }
            }
        };
    }
}
