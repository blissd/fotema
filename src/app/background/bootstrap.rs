// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later
use relm4::{
    Component, ComponentSender,
    WorkerController,
    gtk::glib,
    Worker,
    shared_state::Reducer,
};

use crate::config::APP_ID;
use fotema_core::database;
use fotema_core::photo;
use fotema_core::video;
use fotema_core::visual;

use std::sync::{Arc, Mutex};
use std::time::Instant;

use tracing::{event, Level};

use super::{
    photo_clean::{PhotoClean, PhotoCleanInput, PhotoCleanOutput},
    clean_videos::{CleanVideos, CleanVideosInput, CleanVideosOutput},
    enrich_photos::{EnrichPhotos, EnrichPhotosInput, EnrichPhotosOutput},
    enrich_videos::{EnrichVideos, EnrichVideosInput, EnrichVideosOutput},
    load_library::{LoadLibrary, LoadLibraryInput},
    scan_photos::{ScanPhotos, ScanPhotosInput, ScanPhotosOutput},
    scan_videos::{ScanVideos, ScanVideosInput, ScanVideosOutput},
    thumbnail_photos::{ThumbnailPhotos, ThumbnailPhotosInput, ThumbnailPhotosOutput},
    thumbnail_videos::{ThumbnailVideos, ThumbnailVideosInput, ThumbnailVideosOutput},
};

use crate::app::SharedState;

use crate::app::components::progress_monitor::ProgressMonitor;

/// FIXME copied from progress_monitor. Consolidate?
#[derive(Debug)]
pub enum MediaType {
    Photo,
    Video,
}

/// FIXME very similar (but different) to progress_monitor::TaskName.
/// Any thoughts about this fact?
#[derive(Debug)]
pub enum TaskName {
    Scan(MediaType),
    Enrich(MediaType),
    Thumbnail(MediaType),
    Clean(MediaType),
}

#[derive(Debug)]
pub enum BootstrapInput {
    Start,

    // A background task has started
    TaskStarted(TaskName),

    // A background task has completed
    TaskCompleted(TaskName),
}

#[derive(Debug)]
pub enum BootstrapOutput {
     // Show banner message and start spinner
    TaskStarted(String),

    // Bootstrap process has completed.
    Completed,

}

pub struct Bootstrap {
    started_at: Option<Instant>,

    load_library: WorkerController<LoadLibrary>,

    scan_photos: WorkerController<ScanPhotos>,
    scan_videos: WorkerController<ScanVideos>,

    enrich_photos: WorkerController<EnrichPhotos>,
    enrich_videos: WorkerController<EnrichVideos>,

    photo_clean: WorkerController<PhotoClean>,
    clean_videos: WorkerController<CleanVideos>,

    thumbnail_photos: WorkerController<ThumbnailPhotos>,
    thumbnail_videos: WorkerController<ThumbnailVideos>,
}

impl Worker for Bootstrap {
    type Init = (Arc<Mutex<database::Connection>>, SharedState, Arc<Reducer<ProgressMonitor>>);
    type Input = BootstrapInput;
    type Output = BootstrapOutput;

    fn init((con, state, progress_monitor): Self::Init, sender: ComponentSender<Self>) -> Self  {
        let data_dir = glib::user_data_dir().join(APP_ID);
        let _ = std::fs::create_dir_all(&data_dir);

        let cache_dir = glib::user_cache_dir().join(APP_ID);
        let _ = std::fs::create_dir_all(&cache_dir);

        let pic_base_dir = glib::user_special_dir(glib::enums::UserDirectory::Pictures)
            .expect("Expect XDG_PICTURES_DIR");

        let photo_scan = photo::Scanner::build(&pic_base_dir).unwrap();

        let photo_repo = photo::Repository::open(
            &pic_base_dir,
            &cache_dir,
            con.clone(),
        )
        .unwrap();

        let photo_thumbnailer = photo::Thumbnailer::build(&cache_dir).unwrap();

        let video_scan = video::Scanner::build(&pic_base_dir).unwrap();

        let video_repo = {
            video::Repository::open(&pic_base_dir, &cache_dir, con.clone()).unwrap()
        };

        let video_thumbnailer = video::Thumbnailer::build(&cache_dir).unwrap();

        let visual_repo = visual::Repository::open(
            &pic_base_dir,
            &cache_dir,
            con.clone(),
        )
        .unwrap();

        let load_library = LoadLibrary::builder()
            .detach_worker((visual_repo.clone(), state))
            .detach();

        let scan_photos = ScanPhotos::builder()
            .detach_worker((photo_scan.clone(), photo_repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                ScanPhotosOutput::Started => BootstrapInput::TaskStarted(TaskName::Scan(MediaType::Photo)),
                ScanPhotosOutput::Completed => BootstrapInput::TaskCompleted(TaskName::Scan(MediaType::Photo)),
            });

        let scan_videos = ScanVideos::builder()
            .detach_worker((video_scan.clone(), video_repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                ScanVideosOutput::Started => BootstrapInput::TaskStarted(TaskName::Scan(MediaType::Video)),
                ScanVideosOutput::Completed => BootstrapInput::TaskCompleted(TaskName::Scan(MediaType::Video)),
            });

        let enrich_photos = EnrichPhotos::builder()
            .detach_worker(photo_repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                EnrichPhotosOutput::Started => BootstrapInput::TaskStarted(TaskName::Enrich(MediaType::Photo)),
                EnrichPhotosOutput::Completed => BootstrapInput::TaskCompleted(TaskName::Enrich(MediaType::Photo)),
            });

        let enrich_videos = EnrichVideos::builder()
            .detach_worker(video_repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                EnrichVideosOutput::Started => BootstrapInput::TaskStarted(TaskName::Enrich(MediaType::Video)),
                EnrichVideosOutput::Completed => BootstrapInput::TaskCompleted(TaskName::Enrich(MediaType::Video)),
            });

        let thumbnail_photos = ThumbnailPhotos::builder()
            .detach_worker((photo_thumbnailer.clone(), photo_repo.clone(), progress_monitor.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                ThumbnailPhotosOutput::Started => BootstrapInput::TaskStarted(TaskName::Thumbnail(MediaType::Photo)),
                ThumbnailPhotosOutput::Completed => BootstrapInput::TaskCompleted(TaskName::Thumbnail(MediaType::Photo)),
            });

        let thumbnail_videos = ThumbnailVideos::builder()
            .detach_worker((video_thumbnailer.clone(), video_repo.clone(), progress_monitor.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                ThumbnailVideosOutput::Started => BootstrapInput::TaskStarted(TaskName::Thumbnail(MediaType::Video)),
                ThumbnailVideosOutput::Completed => BootstrapInput::TaskCompleted(TaskName::Thumbnail(MediaType::Video)),
            });

        let photo_clean = PhotoClean::builder()
            .detach_worker(photo_repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                PhotoCleanOutput::Started => BootstrapInput::TaskStarted(TaskName::Clean(MediaType::Photo)),
                PhotoCleanOutput::Completed => BootstrapInput::TaskCompleted(TaskName::Clean(MediaType::Video)),
            });

        let clean_videos = CleanVideos::builder()
            .detach_worker(video_repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                CleanVideosOutput::Started => BootstrapInput::TaskStarted(TaskName::Clean(MediaType::Video)),
                CleanVideosOutput::Completed => BootstrapInput::TaskCompleted(TaskName::Clean(MediaType::Video)),
            });

        let model = Bootstrap {
            started_at: None,
            load_library,
            scan_photos,
            scan_videos,
            enrich_photos,
            enrich_videos,
            photo_clean,
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
                event!(Level::INFO, "bootstrap: start");
                self.started_at = Some(Instant::now());

                // Initial library load to reduce time from starting app and seeing a photo grid
                self.load_library.emit(LoadLibraryInput::Refresh);
                self.scan_photos.emit(ScanPhotosInput::Start);
            }
            BootstrapInput::TaskStarted(TaskName::Scan(MediaType::Photo)) => {
                event!(Level::INFO, "bootstrap: scan photos started");
                let _  = sender.output(BootstrapOutput::TaskStarted(String::from("Scanning file system for photos.")));
            }
            BootstrapInput::TaskCompleted(TaskName::Scan(MediaType::Photo)) => {
                event!(Level::INFO, "bootstrap: scan photos completed");
                self.scan_videos.emit(ScanVideosInput::Start);
            }
            BootstrapInput::TaskStarted(TaskName::Scan(MediaType::Video)) => {
                event!(Level::INFO, "bootstrap: scan videos started");
                let _  = sender.output(BootstrapOutput::TaskStarted(String::from("Scanning file system for videos.")));
            }
            BootstrapInput::TaskCompleted(TaskName::Scan(MediaType::Video)) => {
                event!(Level::INFO, "bootstrap: scan videos completed");
                self.enrich_photos.emit(EnrichPhotosInput::Start);
            }
            BootstrapInput::TaskStarted(TaskName::Enrich(MediaType::Photo)) => {
                event!(Level::INFO, "bootstrap: photo enrichment started");
                let _  = sender.output(BootstrapOutput::TaskStarted(String::from("Processing photo metadata.")));
            }
            BootstrapInput::TaskCompleted(TaskName::Enrich(MediaType::Photo)) => {
                event!(Level::INFO, "bootstrap: photo enrichment completed");
                self.enrich_videos.emit(EnrichVideosInput::Start);
            }
            BootstrapInput::TaskStarted(TaskName::Enrich(MediaType::Video)) => {
                event!(Level::INFO, "bootstrap: video enrichment started");
                let _  = sender.output(BootstrapOutput::TaskStarted(String::from("Processing video metadata.")));
            }
            BootstrapInput::TaskCompleted(TaskName::Enrich(MediaType::Video)) => {
                event!(Level::INFO, "bootstrap: video enrichment completed");

                // metadata might have changed, so reload library
                // FIXME only reload if we know new items were found when scanning,
                // or items had metadata updated
                self.load_library.emit(LoadLibraryInput::Refresh);

                self.thumbnail_photos.emit(ThumbnailPhotosInput::Start);
            }
            BootstrapInput::TaskStarted(TaskName::Thumbnail(MediaType::Photo)) => {
                event!(Level::INFO, "bootstrap: photo thumbnails started");
                let _  = sender.output(BootstrapOutput::TaskStarted(String::from("Generating photo thumbnails. This will take a while.")));
            }
            BootstrapInput::TaskCompleted(TaskName::Thumbnail(MediaType::Photo)) => {
                event!(Level::INFO, "bootstrap: photo thumbnails completed");
                self.thumbnail_videos.emit(ThumbnailVideosInput::Start);
            }
            BootstrapInput::TaskStarted(TaskName::Thumbnail(MediaType::Video)) => {
                event!(Level::INFO, "bootstrap: video thumbnails started");
                let _  = sender.output(BootstrapOutput::TaskStarted(String::from("Generating video thumbnails. This will take a while.")));
            }
            BootstrapInput::TaskCompleted(TaskName::Thumbnail(MediaType::Video)) => {
                let duration = self.started_at.map(|x| x.elapsed());
                event!(Level::INFO, "bootstrap: video thumbnails completed in {:?}", duration);
                self.photo_clean.emit(PhotoCleanInput::Start);
            }
            BootstrapInput::TaskStarted(TaskName::Clean(MediaType::Photo)) => {
                event!(Level::INFO, "bootstrap: photo cleanup started.");
                let _  = sender.output(BootstrapOutput::TaskStarted(String::from("Photo database maintenance.")));
            }
            BootstrapInput::TaskCompleted(TaskName::Clean(MediaType::Photo)) => {
                event!(Level::INFO, "bootstrap: photo cleanup completed.");
                self.clean_videos.emit(CleanVideosInput::Start);
            }
            BootstrapInput::TaskStarted(TaskName::Clean(MediaType::Video)) => {
                event!(Level::INFO, "bootstrap: video cleanup started.");
                let _  = sender.output(BootstrapOutput::TaskStarted(String::from("Video database maintenance.")));
            }
            BootstrapInput::TaskCompleted(TaskName::Clean(MediaType::Video)) => {
                event!(Level::INFO, "bootstrap: video cleanup completed.");
                let _ = sender.output(BootstrapOutput::Completed);
            }
        };
    }
}
