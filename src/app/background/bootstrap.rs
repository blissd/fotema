// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later
use relm4::{
    Component, ComponentSender,
    WorkerController,
    gtk::glib,
    Worker,
};
use crate::config::APP_ID;
use fotema_core::database;
use fotema_core::video;

use std::sync::{Arc, Mutex};

use super::{
    clean_photos::{CleanPhotos, CleanPhotosInput, CleanPhotosOutput},
    clean_videos::{CleanVideos, CleanVideosInput, CleanVideosOutput},
    enrich_photos::{EnrichPhotos, EnrichPhotosInput, EnrichPhotosOutput},
    enrich_videos::{EnrichVideos, EnrichVideosInput, EnrichVideosOutput},
    load_library::{LoadLibrary, LoadLibraryInput, LoadLibraryOutput},
    scan_photos::{ScanPhotos, ScanPhotosInput, ScanPhotosOutput},
    scan_videos::{ScanVideos, ScanVideosInput, ScanVideosOutput},
    thumbnail_photos::{ThumbnailPhotos, ThumbnailPhotosInput, ThumbnailPhotosOutput},
    thumbnail_videos::{ThumbnailVideos, ThumbnailVideosInput, ThumbnailVideosOutput},
};

#[derive(Debug)]
pub enum BootstrapInput {
    Start,

    // Photos library has been loaded from database.
    LibraryRefreshed,

    // File-system scan events
    PhotoScanStarted,
    PhotoScanCompleted,

    VideoScanStarted,
    VideoScanCompleted,

    // Enrich with metadata

    PhotoEnrichmentStarted(usize),
    PhotoEnrichmentCompleted,

    VideoEnrichmentStarted(usize),
    VideoEnrichmentCompleted,

    // Thumbnail generation events

    ThumbnailPhotosStarted(usize),
    ThumbnailPhotosGenerated,
    ThumbnailPhotosCompleted,

    ThumbnailVideosStarted(usize),
    ThumbnailVideosGenerated,
    ThumbnailVideosCompleted,

    // Cleanup events
    PhotoCleanStarted,
    PhotoCleanCompleted,

    VideoCleanStarted,
    VideoCleanCompleted,
}

#[derive(Debug)]
pub enum BootstrapOutput {
    // A task that can make progress has started.
    // count of items, banner text, progress bar text
    ProgressStarted(usize, String, String),

    // One item has been processed
    ProgressAdvanced,

    // Finished processing
    ProgressCompleted,

    // Show banner message and start spinner
    TaskStarted(String),

    // Library has been refreshed.
    LibraryRefreshed,

    // Bootstrap process has completed.
    Completed,

}

pub struct Bootstrap {
    load_library: WorkerController<LoadLibrary>,

    scan_photos: WorkerController<ScanPhotos>,
    scan_videos: WorkerController<ScanVideos>,

    enrich_photos: WorkerController<EnrichPhotos>,
    enrich_videos: WorkerController<EnrichVideos>,

    clean_photos: WorkerController<CleanPhotos>,
    clean_videos: WorkerController<CleanVideos>,

    thumbnail_photos: WorkerController<ThumbnailPhotos>,
    thumbnail_videos: WorkerController<ThumbnailVideos>,
}

impl Worker for Bootstrap {
    type Init = (Arc<Mutex<database::Connection>>, fotema_core::Library);
    type Input = BootstrapInput;
    type Output = BootstrapOutput;

    fn init((con, library): Self::Init, sender: ComponentSender<Self>) -> Self  {
        let data_dir = glib::user_data_dir().join(APP_ID);
        let _ = std::fs::create_dir_all(&data_dir);

        let cache_dir = glib::user_cache_dir().join(APP_ID);
        let _ = std::fs::create_dir_all(&cache_dir);

        let pic_base_dir = glib::user_special_dir(glib::enums::UserDirectory::Pictures)
            .expect("Expect XDG_PICTURES_DIR");

        let photo_scan = fotema_core::photo::Scanner::build(&pic_base_dir).unwrap();

        let photo_repo = fotema_core::photo::Repository::open(
            &pic_base_dir,
            &cache_dir,
            con.clone(),
        )
        .unwrap();

        let photo_thumbnailer = fotema_core::photo::Thumbnailer::build(&cache_dir).unwrap();

        let video_scan = fotema_core::video::Scanner::build(&pic_base_dir).unwrap();

        let video_repo = {
            video::Repository::open(&pic_base_dir, &cache_dir, con.clone()).unwrap()
        };

        let video_transcoder = video::Transcoder::new(&cache_dir);

        let video_thumbnailer = fotema_core::video::Thumbnailer::build(&cache_dir).unwrap();

        let load_library = LoadLibrary::builder()
            .detach_worker(library.clone())
            .forward(sender.input_sender(), |msg| match msg {
                LoadLibraryOutput::Refreshed => BootstrapInput::LibraryRefreshed,
            });

        let scan_photos = ScanPhotos::builder()
            .detach_worker((photo_scan.clone(), photo_repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                ScanPhotosOutput::Started => BootstrapInput::PhotoScanStarted,
                ScanPhotosOutput::Completed => BootstrapInput::PhotoScanCompleted,
            });

        let scan_videos = ScanVideos::builder()
            .detach_worker((video_scan.clone(), video_repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                ScanVideosOutput::Started => BootstrapInput::VideoScanStarted,
                ScanVideosOutput::Completed => BootstrapInput::VideoScanCompleted,
            });

        let enrich_videos = EnrichVideos::builder()
            .detach_worker(video_repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                EnrichVideosOutput::Started(count) => BootstrapInput::VideoEnrichmentStarted(count),
                EnrichVideosOutput::Completed => BootstrapInput::VideoEnrichmentCompleted,
            });

        let enrich_photos = EnrichPhotos::builder()
            .detach_worker(photo_repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                EnrichPhotosOutput::Started(count) => BootstrapInput::PhotoEnrichmentStarted(count),
                EnrichPhotosOutput::Completed => BootstrapInput::PhotoEnrichmentCompleted,
            });

        let thumbnail_photos = ThumbnailPhotos::builder()
            .detach_worker((photo_thumbnailer.clone(), photo_repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                ThumbnailPhotosOutput::Started(count) => BootstrapInput::ThumbnailPhotosStarted(count),
                ThumbnailPhotosOutput::Generated => BootstrapInput::ThumbnailPhotosGenerated,
                ThumbnailPhotosOutput::Completed => BootstrapInput::ThumbnailPhotosCompleted,
            });

        let thumbnail_videos = ThumbnailVideos::builder()
            .detach_worker((video_thumbnailer.clone(), video_repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                ThumbnailVideosOutput::Started(count) => BootstrapInput::ThumbnailVideosStarted(count),
                ThumbnailVideosOutput::Generated => BootstrapInput::ThumbnailVideosGenerated,
                ThumbnailVideosOutput::Completed => BootstrapInput::ThumbnailVideosCompleted,
            });

        let clean_photos = CleanPhotos::builder()
            .detach_worker(photo_repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                CleanPhotosOutput::Started => BootstrapInput::PhotoCleanStarted,
                CleanPhotosOutput::Completed => BootstrapInput::PhotoCleanCompleted,
            });

        let clean_videos = CleanVideos::builder()
            .detach_worker(video_repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                CleanVideosOutput::Started => BootstrapInput::VideoCleanStarted,
                CleanVideosOutput::Completed => BootstrapInput::VideoCleanCompleted,
            });

        let model = Bootstrap {
            load_library,
            scan_photos,
            scan_videos,
            enrich_photos,
            enrich_videos,
            clean_photos,
            clean_videos,
            thumbnail_photos,
            thumbnail_videos,
        };
        model
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        // This match block coordinates the background tasks launched immediately after
        // the app starts up.
        match msg {
            BootstrapInput::Start => {
                println!("bootstrap: start");
                self.scan_photos.emit(ScanPhotosInput::Start);
            }
            BootstrapInput::PhotoScanStarted => {
                println!("bootstrap: scan photos started");
                let _  = sender.output(BootstrapOutput::TaskStarted(String::from("Scanning file system for photos.")));
            }
            BootstrapInput::PhotoScanCompleted => {
                println!("bootstrap: scan photos completed");
                self.scan_videos.emit(ScanVideosInput::Start);
            }
            BootstrapInput::VideoScanStarted => {
                println!("bootstrap: scan videos started");
                let _  = sender.output(BootstrapOutput::TaskStarted(String::from("Scanning file system for videos.")));
            }
            BootstrapInput::VideoScanCompleted => {
                println!("bootstrap: scan videos completed");
                self.clean_photos.emit(CleanPhotosInput::Start);
            }
            BootstrapInput::PhotoCleanStarted => {
                println!("bootstrap: photo cleanup started.");
                let _  = sender.output(BootstrapOutput::TaskStarted(String::from("Photo database maintenance.")));
            }
            BootstrapInput::PhotoCleanCompleted => {
                println!("bootstrap: photo cleanup completed.");
                self.clean_videos.emit(CleanVideosInput::Start);
            }
            BootstrapInput::VideoCleanStarted => {
                println!("bootstrap: video cleanup started.");
                let _  = sender.output(BootstrapOutput::TaskStarted(String::from("Video database maintenance.")));
            }
            BootstrapInput::VideoCleanCompleted => {
                println!("bootstrap: video cleanup completed.");
                self.enrich_photos.emit(EnrichPhotosInput::Start);
            }
            BootstrapInput::PhotoEnrichmentStarted(count) => {
                println!("bootstrap: photo enrichment started");
                let msg = "Processing photo metadata.".to_string();
                let _ = sender.output(BootstrapOutput::ProgressStarted(count, msg.clone(), msg));
            }
            BootstrapInput::PhotoEnrichmentCompleted => {
                println!("bootstrap: photo enrichment completed");
                let _ = sender.output(BootstrapOutput::ProgressCompleted);
                self.enrich_videos.emit(EnrichVideosInput::Start);
            }
            BootstrapInput::VideoEnrichmentStarted(count) => {
                println!("bootstrap: video enrichment started");
                let msg = "Processing video metadata.".to_string();
                let _ = sender.output(BootstrapOutput::ProgressStarted(count, msg.clone(), msg));
            }
            BootstrapInput::VideoEnrichmentCompleted => {
                println!("bootstrap: video enrichment completed");
                let _ = sender.output(BootstrapOutput::ProgressCompleted);
                self.load_library.emit(LoadLibraryInput::Refresh);
                self.thumbnail_photos.emit(ThumbnailPhotosInput::Start);
            }
            BootstrapInput::ThumbnailPhotosStarted(count) => {
                println!("bootstrap: photo thumbnails started");
                let banner = "Generating photo thumbnails. This will take a while.".to_string();
                let progress_text = "Generating photo thumbnails.".to_string();
                let _ = sender.output(BootstrapOutput::ProgressStarted(count, banner, progress_text));
            }
            BootstrapInput::ThumbnailPhotosGenerated => {
                println!("bootstrap: photo thumbnails advanced");
                let _ = sender.output(BootstrapOutput::ProgressAdvanced);
            }
            BootstrapInput::ThumbnailPhotosCompleted => {
                println!("bootstrap: photo thumbnails completed");
                let _ = sender.output(BootstrapOutput::ProgressCompleted);
                self.thumbnail_videos.emit(ThumbnailVideosInput::Start);
            }
            BootstrapInput::ThumbnailVideosStarted(count) => {
                println!("bootstrap: video thumbnails started");
                let banner = "Generating video thumbnails. This will take a while.".to_string();
                let progress_text = "Generating video thumbnails.".to_string();
                let _ = sender.output(BootstrapOutput::ProgressStarted(count, banner, progress_text));
            }
            BootstrapInput::ThumbnailVideosGenerated => {
                println!("bootstrap: photo thumbnails advanced");
                let _ = sender.output(BootstrapOutput::ProgressAdvanced);
            }
            BootstrapInput::ThumbnailVideosCompleted => {
                println!("bootstrap: video thumbnails completed");
                let _ = sender.output(BootstrapOutput::ProgressCompleted);
                let _ = sender.output(BootstrapOutput::Completed);

            }
            BootstrapInput::LibraryRefreshed => {
                let _ = sender.output(BootstrapOutput::LibraryRefreshed);
            }
        };
    }
}
