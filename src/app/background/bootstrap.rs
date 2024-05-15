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
    load_library::{LoadLibrary, LoadLibraryInput},

    photo_clean::{PhotoClean, PhotoCleanInput, PhotoCleanOutput},
    photo_enrich::{PhotoEnrich, PhotoEnrichInput, PhotoEnrichOutput},
    photo_scan::{PhotoScan, PhotoScanInput, PhotoScanOutput},
    photo_thumbnail::{PhotoThumbnail, PhotoThumbnailInput, PhotoThumbnailOutput},

    video_clean::{VideoClean, VideoCleanInput, VideoCleanOutput},
    video_enrich::{VideoEnrich, VideoEnrichInput, VideoEnrichOutput},
    video_scan::{VideoScan, VideoScanInput, VideoScanOutput},
    video_thumbnail::{VideoThumbnail, VideoThumbnailInput, VideoThumbnailOutput},
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

    photo_scan: WorkerController<PhotoScan>,
    video_scan: WorkerController<VideoScan>,

    photo_enrich: WorkerController<PhotoEnrich>,
    video_enrich: WorkerController<VideoEnrich>,

    photo_clean: WorkerController<PhotoClean>,
    video_clean: WorkerController<VideoClean>,

    photo_thumbnail: WorkerController<PhotoThumbnail>,
    video_thumbnail: WorkerController<VideoThumbnail>,
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

        let photo_scanner = photo::Scanner::build(&pic_base_dir).unwrap();

        let photo_repo = photo::Repository::open(
            &pic_base_dir,
            &cache_dir,
            con.clone(),
        )
        .unwrap();

        let photo_thumbnailer = photo::Thumbnailer::build(&cache_dir).unwrap();

        let video_scanner = video::Scanner::build(&pic_base_dir).unwrap();

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

        let photo_scan = PhotoScan::builder()
            .detach_worker((photo_scanner.clone(), photo_repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                PhotoScanOutput::Started => BootstrapInput::TaskStarted(TaskName::Scan(MediaType::Photo)),
                PhotoScanOutput::Completed => BootstrapInput::TaskCompleted(TaskName::Scan(MediaType::Photo)),
            });

        let video_scan = VideoScan::builder()
            .detach_worker((video_scanner.clone(), video_repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                VideoScanOutput::Started => BootstrapInput::TaskStarted(TaskName::Scan(MediaType::Video)),
                VideoScanOutput::Completed => BootstrapInput::TaskCompleted(TaskName::Scan(MediaType::Video)),
            });

        let photo_enrich = PhotoEnrich::builder()
            .detach_worker(photo_repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                PhotoEnrichOutput::Started => BootstrapInput::TaskStarted(TaskName::Enrich(MediaType::Photo)),
                PhotoEnrichOutput::Completed => BootstrapInput::TaskCompleted(TaskName::Enrich(MediaType::Photo)),
            });

        let video_enrich = VideoEnrich::builder()
            .detach_worker((video_repo.clone(), progress_monitor.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                VideoEnrichOutput::Started => BootstrapInput::TaskStarted(TaskName::Enrich(MediaType::Video)),
                VideoEnrichOutput::Completed => BootstrapInput::TaskCompleted(TaskName::Enrich(MediaType::Video)),
            });

        let photo_thumbnail = PhotoThumbnail::builder()
            .detach_worker((photo_thumbnailer.clone(), photo_repo.clone(), progress_monitor.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                PhotoThumbnailOutput::Started => BootstrapInput::TaskStarted(TaskName::Thumbnail(MediaType::Photo)),
                PhotoThumbnailOutput::Completed => BootstrapInput::TaskCompleted(TaskName::Thumbnail(MediaType::Photo)),
            });

        let video_thumbnail = VideoThumbnail::builder()
            .detach_worker((video_thumbnailer.clone(), video_repo.clone(), progress_monitor.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                VideoThumbnailOutput::Started => BootstrapInput::TaskStarted(TaskName::Thumbnail(MediaType::Video)),
                VideoThumbnailOutput::Completed => BootstrapInput::TaskCompleted(TaskName::Thumbnail(MediaType::Video)),
            });

        let photo_clean = PhotoClean::builder()
            .detach_worker(photo_repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                PhotoCleanOutput::Started => BootstrapInput::TaskStarted(TaskName::Clean(MediaType::Photo)),
                PhotoCleanOutput::Completed => BootstrapInput::TaskCompleted(TaskName::Clean(MediaType::Video)),
            });

        let video_clean = VideoClean::builder()
            .detach_worker(video_repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                VideoCleanOutput::Started => BootstrapInput::TaskStarted(TaskName::Clean(MediaType::Video)),
                VideoCleanOutput::Completed => BootstrapInput::TaskCompleted(TaskName::Clean(MediaType::Video)),
            });

        let model = Bootstrap {
            started_at: None,
            load_library,
            photo_scan,
            video_scan,
            photo_enrich,
            video_enrich,
            photo_clean,
            video_clean,
            photo_thumbnail,
            video_thumbnail,
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
                self.photo_scan.emit(PhotoScanInput::Start);
            }
            BootstrapInput::TaskStarted(TaskName::Scan(MediaType::Photo)) => {
                event!(Level::INFO, "bootstrap: scan photos started");
                let _  = sender.output(BootstrapOutput::TaskStarted(String::from("Scanning file system for photos.")));
            }
            BootstrapInput::TaskCompleted(TaskName::Scan(MediaType::Photo)) => {
                event!(Level::INFO, "bootstrap: scan photos completed");
                self.video_scan.emit(VideoScanInput::Start);
            }
            BootstrapInput::TaskStarted(TaskName::Scan(MediaType::Video)) => {
                event!(Level::INFO, "bootstrap: scan videos started");
                let _  = sender.output(BootstrapOutput::TaskStarted(String::from("Scanning file system for videos.")));
            }
            BootstrapInput::TaskCompleted(TaskName::Scan(MediaType::Video)) => {
                event!(Level::INFO, "bootstrap: scan videos completed");
                self.photo_enrich.emit(PhotoEnrichInput::Start);
            }
            BootstrapInput::TaskStarted(TaskName::Enrich(MediaType::Photo)) => {
                event!(Level::INFO, "bootstrap: photo enrichment started");
                let _  = sender.output(BootstrapOutput::TaskStarted(String::from("Processing photo metadata.")));
            }
            BootstrapInput::TaskCompleted(TaskName::Enrich(MediaType::Photo)) => {
                event!(Level::INFO, "bootstrap: photo enrichment completed");
                self.video_enrich.emit(VideoEnrichInput::Start);
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

                self.photo_thumbnail.emit(PhotoThumbnailInput::Start);
            }
            BootstrapInput::TaskStarted(TaskName::Thumbnail(MediaType::Photo)) => {
                event!(Level::INFO, "bootstrap: photo thumbnails started");
                let _  = sender.output(BootstrapOutput::TaskStarted(String::from("Generating photo thumbnails. This will take a while.")));
            }
            BootstrapInput::TaskCompleted(TaskName::Thumbnail(MediaType::Photo)) => {
                event!(Level::INFO, "bootstrap: photo thumbnails completed");
                self.video_thumbnail.emit(VideoThumbnailInput::Start);
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
                self.video_clean.emit(VideoCleanInput::Start);
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
