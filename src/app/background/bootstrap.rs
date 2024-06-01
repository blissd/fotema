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

use tracing::info;

use super::{
    load_library::{LoadLibrary, LoadLibraryInput},

    photo_clean::{PhotoClean, PhotoCleanInput, PhotoCleanOutput},
    photo_enrich::{PhotoEnrich, PhotoEnrichInput, PhotoEnrichOutput},
    photo_scan::{PhotoScan, PhotoScanInput, PhotoScanOutput},
    photo_thumbnail::{PhotoThumbnail, PhotoThumbnailInput, PhotoThumbnailOutput},
    photo_extract_motion::{PhotoExtractMotion, PhotoExtractMotionInput, PhotoExtractMotionOutput},

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
    MotionPhoto,
    Thumbnail(MediaType),
    Clean(MediaType),
}

#[derive(Debug)]
pub enum BootstrapInput {
    Start,

    // A background task has started
    TaskStarted(TaskName),

    // A background task has completed.
    // usize is count of processed items.
    TaskCompleted(TaskName, Option<usize>),
}

#[derive(Debug)]
pub enum BootstrapOutput {
     // Show banner message and start spinner
    TaskStarted(TaskName),

    // Bootstrap process has completed.
    Completed,

}

pub struct Bootstrap {
    started_at: Option<Instant>,

    /// Whether a background task has updated some library state and the library should be reloaded.
    library_stale: bool,

    load_library: WorkerController<LoadLibrary>,

    photo_scan: WorkerController<PhotoScan>,
    video_scan: WorkerController<VideoScan>,

    photo_enrich: WorkerController<PhotoEnrich>,
    video_enrich: WorkerController<VideoEnrich>,

    photo_clean: WorkerController<PhotoClean>,
    video_clean: WorkerController<VideoClean>,

    photo_thumbnail: WorkerController<PhotoThumbnail>,
    video_thumbnail: WorkerController<VideoThumbnail>,

    photo_extract_motion: WorkerController<PhotoExtractMotion>,
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

        let motion_photo_extractor = photo::MotionPhotoExtractor::build(&cache_dir).unwrap();

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
                PhotoScanOutput::Completed => BootstrapInput::TaskCompleted(TaskName::Scan(MediaType::Photo), None),
            });

        let video_scan = VideoScan::builder()
            .detach_worker((video_scanner.clone(), video_repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                VideoScanOutput::Started => BootstrapInput::TaskStarted(TaskName::Scan(MediaType::Video)),
                VideoScanOutput::Completed => BootstrapInput::TaskCompleted(TaskName::Scan(MediaType::Video), None),
            });

        let photo_enrich = PhotoEnrich::builder()
            .detach_worker(photo_repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                PhotoEnrichOutput::Started => BootstrapInput::TaskStarted(TaskName::Enrich(MediaType::Photo)),
                PhotoEnrichOutput::Completed(count) => BootstrapInput::TaskCompleted(TaskName::Enrich(MediaType::Photo), Some(count)),
            });

        let video_enrich = VideoEnrich::builder()
            .detach_worker((video_repo.clone(), progress_monitor.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                VideoEnrichOutput::Started => BootstrapInput::TaskStarted(TaskName::Enrich(MediaType::Video)),
                VideoEnrichOutput::Completed(count) => BootstrapInput::TaskCompleted(TaskName::Enrich(MediaType::Video), Some(count)),
            });

        let photo_extract_motion = PhotoExtractMotion::builder()
            .detach_worker((motion_photo_extractor, photo_repo.clone(), progress_monitor.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                PhotoExtractMotionOutput::Started => BootstrapInput::TaskStarted(TaskName::MotionPhoto),
                PhotoExtractMotionOutput::Completed(count) => BootstrapInput::TaskCompleted(TaskName::MotionPhoto, Some(count)),
            });

        let photo_thumbnail = PhotoThumbnail::builder()
            .detach_worker((photo_thumbnailer.clone(), photo_repo.clone(), progress_monitor.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                PhotoThumbnailOutput::Started => BootstrapInput::TaskStarted(TaskName::Thumbnail(MediaType::Photo)),
                PhotoThumbnailOutput::Completed(count) => BootstrapInput::TaskCompleted(TaskName::Thumbnail(MediaType::Photo), Some(count)),
            });

        let video_thumbnail = VideoThumbnail::builder()
            .detach_worker((video_thumbnailer.clone(), video_repo.clone(), progress_monitor.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                VideoThumbnailOutput::Started => BootstrapInput::TaskStarted(TaskName::Thumbnail(MediaType::Video)),
                VideoThumbnailOutput::Completed(count) => BootstrapInput::TaskCompleted(TaskName::Thumbnail(MediaType::Video), Some(count)),
            });

        let photo_clean = PhotoClean::builder()
            .detach_worker(photo_repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                PhotoCleanOutput::Started => BootstrapInput::TaskStarted(TaskName::Clean(MediaType::Photo)),
                PhotoCleanOutput::Completed(count) => BootstrapInput::TaskCompleted(TaskName::Clean(MediaType::Video), Some(count)),
            });

        let video_clean = VideoClean::builder()
            .detach_worker(video_repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                VideoCleanOutput::Started => BootstrapInput::TaskStarted(TaskName::Clean(MediaType::Video)),
                VideoCleanOutput::Completed(count) => BootstrapInput::TaskCompleted(TaskName::Clean(MediaType::Video), Some(count)),
            });

        let model = Bootstrap {
            started_at: None,
            library_stale: false,
            load_library,
            photo_scan,
            video_scan,
            photo_enrich,
            video_enrich,
            photo_extract_motion,
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
                info!("Start");
                self.started_at = Some(Instant::now());

                // Initial library load to reduce time from starting app and seeing a photo grid
                self.load_library.emit(LoadLibraryInput::Refresh);
                self.photo_scan.emit(PhotoScanInput::Start);
            }
            BootstrapInput::TaskStarted(task_name @ TaskName::Scan(MediaType::Photo)) => {
                info!("Scan photos started");
                let _  = sender.output(BootstrapOutput::TaskStarted(task_name));
            }
            BootstrapInput::TaskCompleted(TaskName::Scan(MediaType::Photo), updated) => {
                info!("Scan photos completed");
                self.library_stale = self.library_stale || updated.is_some_and(|x| x > 0);
                self.video_scan.emit(VideoScanInput::Start);
            }
            BootstrapInput::TaskStarted(task_name @ TaskName::Scan(MediaType::Video)) => {
                info!("Scan videos started");
                let _  = sender.output(BootstrapOutput::TaskStarted(task_name));
            }
            BootstrapInput::TaskCompleted(TaskName::Scan(MediaType::Video), updated) => {
                info!("Scan videos completed");
                self.library_stale = self.library_stale || updated.is_some_and(|x| x > 0);
                self.photo_enrich.emit(PhotoEnrichInput::Start);
            }
            BootstrapInput::TaskStarted(task_name @ TaskName::Enrich(MediaType::Photo)) => {
                info!("Photo enrichment started");
                let _  = sender.output(BootstrapOutput::TaskStarted(task_name));
            }
            BootstrapInput::TaskCompleted(TaskName::Enrich(MediaType::Photo), updated) => {
                info!("Photo enrichment completed");
                self.library_stale = self.library_stale || updated.is_some_and(|x| x > 0);
                self.video_enrich.emit(VideoEnrichInput::Start);
            }
            BootstrapInput::TaskStarted(task_name @ TaskName::Enrich(MediaType::Video)) => {
                info!("Video enrichment started");
                let _  = sender.output(BootstrapOutput::TaskStarted(task_name));
            }
            BootstrapInput::TaskCompleted(TaskName::Enrich(MediaType::Video), updated) => {
                info!("Video enrichment completed");

                self.library_stale = self.library_stale || updated.is_some_and(|x| x > 0);
                if self.library_stale {
                    info!("Refreshing library after video enrichment");
                    self.load_library.emit(LoadLibraryInput::Refresh);
                }
                self.library_stale = false;

                self.photo_extract_motion.emit(PhotoExtractMotionInput::Start);
            }
            BootstrapInput::TaskStarted(task_name @ TaskName::MotionPhoto) => {
                info!("Motion photo extract started");
                let _  = sender.output(BootstrapOutput::TaskStarted(task_name));
            }
            BootstrapInput::TaskCompleted(TaskName::MotionPhoto, updated) => {
                info!("photo thumbnails completed");
                self.library_stale = self.library_stale || updated.is_some_and(|x| x > 0);
                self.photo_thumbnail.emit(PhotoThumbnailInput::Start);
            }
            BootstrapInput::TaskStarted(task_name @ TaskName::Thumbnail(MediaType::Photo)) => {
                info!("Photo thumbnails started");
                let _  = sender.output(BootstrapOutput::TaskStarted(task_name));
            }
            BootstrapInput::TaskCompleted(TaskName::Thumbnail(MediaType::Photo), updated) => {
                info!("Photo thumbnails completed");
                self.library_stale = self.library_stale || updated.is_some_and(|x| x > 0);
                self.video_thumbnail.emit(VideoThumbnailInput::Start);
            }
            BootstrapInput::TaskStarted(task_name @ TaskName::Thumbnail(MediaType::Video)) => {
                info!("Video thumbnails started");
                let _  = sender.output(BootstrapOutput::TaskStarted(task_name));
            }
            BootstrapInput::TaskCompleted(TaskName::Thumbnail(MediaType::Video), updated) => {
                let duration = self.started_at.map(|x| x.elapsed());
                info!("Video thumbnails completed in {:?}", duration);
                self.library_stale = self.library_stale || updated.is_some_and(|x| x > 0);
                self.photo_clean.emit(PhotoCleanInput::Start);
            }
            BootstrapInput::TaskStarted(task_name @ TaskName::Clean(MediaType::Photo)) => {
                info!("Photo cleanup started.");
                let _  = sender.output(BootstrapOutput::TaskStarted(task_name));
            }
            BootstrapInput::TaskCompleted(TaskName::Clean(MediaType::Photo), updated) => {
                info!("Photo cleanup completed.");
                self.library_stale = self.library_stale || updated.is_some_and(|x| x > 0);
                self.video_clean.emit(VideoCleanInput::Start);
            }
            BootstrapInput::TaskStarted(task_name @ TaskName::Clean(MediaType::Video)) => {
                info!("Video cleanup started.");
                let _  = sender.output(BootstrapOutput::TaskStarted(task_name));
            }
            BootstrapInput::TaskCompleted(TaskName::Clean(MediaType::Video), updated) => {
                info!("Video cleanup completed.");

                // This is the last background task to complete. Refresh library if there
                // has been a visible change to the library state.
                self.library_stale = self.library_stale || updated.is_some_and(|x| x > 0);
                if self.library_stale {
                    info!("Refreshing library after video cleanup");
                    self.load_library.emit(LoadLibraryInput::Refresh);
                }
                self.library_stale = false;

                let _ = sender.output(BootstrapOutput::Completed);
            }
        };
    }
}
