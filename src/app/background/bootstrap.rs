// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later
use relm4::{
    Component, ComponentSender, Sender, Worker, WorkerController, gtk::glib, shared_state::Reducer,
};

use crate::app::Settings;
use crate::config::APP_ID;
use fotema_core::PictureId;
use fotema_core::database;
use fotema_core::people;
use fotema_core::photo;
use fotema_core::thumbnailify::Thumbnailer;
use fotema_core::video;
use fotema_core::visual;

use std::result::Result::Ok;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

use tracing::{error, info, warn};

use anyhow;

use super::{
    load_library_task::{LoadLibraryTask, LoadLibraryTaskInput, LoadLibraryTaskOutput},
    photo_clean_task::{PhotoCleanTask, PhotoCleanTaskInput, PhotoCleanTaskOutput},
    photo_detect_faces_task::{
        PhotoDetectFacesTask, PhotoDetectFacesTaskInput, PhotoDetectFacesTaskOutput,
    },
    photo_enrich_task::{PhotoEnrichTask, PhotoEnrichTaskInput, PhotoEnrichTaskOutput},
    photo_extract_motion_task::{
        PhotoExtractMotionTask, PhotoExtractMotionTaskInput, PhotoExtractMotionTaskOutput,
    },
    photo_recognize_faces_task::{
        PhotoRecognizeFacesTask, PhotoRecognizeFacesTaskInput, PhotoRecognizeFacesTaskOutput,
    },
    photo_scan_task::{PhotoScanTask, PhotoScanTaskInput, PhotoScanTaskOutput},
    photo_thumbnail_task::{PhotoThumbnailTask, PhotoThumbnailTaskInput, PhotoThumbnailTaskOutput},
    video_clean_task::{VideoCleanTask, VideoCleanTaskInput, VideoCleanTaskOutput},
    video_enrich_task::{VideoEnrichTask, VideoEnrichTaskInput, VideoEnrichTaskOutput},
    video_scan_task::{VideoScanTask, VideoScanTaskInput, VideoScanTaskOutput},
    video_thumbnail_task::{VideoThumbnailTask, VideoThumbnailTaskInput, VideoThumbnailTaskOutput},
    video_transcode_task::{VideoTranscodeTask, VideoTranscodeTaskInput, VideoTranscodeTaskOutput},

    tidy_task::{TidyTask, TidyTaskInput, TidyTaskOutput},
};

use crate::app::FaceDetectionMode;
use crate::app::SettingsState;
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
    LoadLibrary,
    Scan(MediaType),
    Enrich(MediaType),
    MotionPhoto,
    Thumbnail(MediaType),
    Clean(MediaType),
    DetectFaces,
    RecognizeFaces,
    Transcode,
    Tidy,
}

#[derive(Debug)]
pub enum BootstrapInput {
    /// Configure the pictures library root and host path
    Configure(PathBuf, PathBuf),

    /// Settings updated
    SettingsUpdated(Settings),

    /// Start the initial background processes for setting up Fotema.
    Start,

    // Stop all background tasks
    Stop,

    /// No more tasks running
    Stopped,

    /// Queue task for scanning picture for more faces.
    ScanPictureForFaces(PictureId),
    ScanPicturesForFaces,

    // Queue task for transcoding videos
    TranscodeAll,

    /// A background task has started.
    TaskStarted(TaskName),

    /// A background task has completed.
    /// usize is count of processed items.
    TaskCompleted(TaskName, Option<usize>),
}

#[derive(Debug)]
pub enum BootstrapOutput {
    // Show banner message and start spinner
    TaskStarted(TaskName),

    // Bootstrap process has completed.
    Completed,

    // Tasks are in the process of stopping
    Stopping,
}

type Task = dyn Fn() + Send + Sync;

/// All controllers for running background tasks.
/// TODO: figure out why have a I used Arc here. Can it go?
pub struct Controllers {
    started_at: Option<Instant>,

    shared_state: SharedState,

    settings_state: SettingsState,

    // Stop background tasks.
    stop: Arc<AtomicBool>,

    /// Whether a background task has updated some library state and the library should be reloaded.
    library_stale: Arc<AtomicBool>,

    load_library_task: Arc<WorkerController<LoadLibraryTask>>,

    photo_scan_task: Arc<WorkerController<PhotoScanTask>>,
    video_scan_task: Arc<WorkerController<VideoScanTask>>,

    photo_enrich_task: Arc<WorkerController<PhotoEnrichTask>>,
    video_enrich_task: Arc<WorkerController<VideoEnrichTask>>,

    photo_clean_task: Arc<WorkerController<PhotoCleanTask>>,
    video_clean_task: Arc<WorkerController<VideoCleanTask>>,

    photo_thumbnail_task: Arc<WorkerController<PhotoThumbnailTask>>,
    video_thumbnail_task: Arc<WorkerController<VideoThumbnailTask>>,

    photo_extract_motion_task: Arc<WorkerController<PhotoExtractMotionTask>>,

    photo_detect_faces_task: Arc<WorkerController<PhotoDetectFacesTask>>,
    photo_recognize_faces_task: Arc<WorkerController<PhotoRecognizeFacesTask>>,

    video_transcode_task: Arc<WorkerController<VideoTranscodeTask>>,

    tidy_task: Arc<WorkerController<TidyTask>>,

    /// Pending ordered tasks to process
    /// Wow... figuring out a type signature that would compile was a nightmare.
    pending_tasks: Arc<Mutex<VecDeque<Box<Task>>>>,

    // Is a task currently running?
    is_running: bool,
}

impl Controllers {
    fn update(&mut self, msg: BootstrapInput, sender: ComponentSender<Bootstrap>) {
        // This match block coordinates the background tasks launched immediately after
        // the app starts up.
        match msg {
            BootstrapInput::Start => {
                info!("Start");
                self.started_at = Some(Instant::now());

                if let Ok(mut tasks) = self.pending_tasks.lock() {
                    if let Some(task) = tasks.pop_front() {
                        self.is_running = true;
                        task();
                    } else {
                        self.is_running = false;
                        let _ = sender.output(BootstrapOutput::Completed);
                    }
                }
            }
            BootstrapInput::ScanPictureForFaces(picture_id) => {
                info!("Queueing task to scan picture {} for faces", picture_id);
                self.add_task_photo_detect_faces_for_one(picture_id);
                self.add_task_photo_recognize_faces();
                self.run_if_idle();
            }
            BootstrapInput::ScanPicturesForFaces => {
                info!("Queueing task to scan all pictures for faces");
                self.add_task_photo_detect_faces();
                self.add_task_photo_recognize_faces();
                self.run_if_idle();
            }
            BootstrapInput::TranscodeAll => {
                info!("Queueing task to transcode all incompatible videos");
                self.add_task_video_transcode();
                self.run_if_idle();
            }
            BootstrapInput::TaskStarted(task_name) => {
                info!("Task started: {:?}", task_name);
                let _ = sender.output(BootstrapOutput::TaskStarted(task_name));
            }
            BootstrapInput::TaskCompleted(task_name, updated) => {
                info!(
                    "Task completed: {:?}. Items updated? {:?}",
                    task_name, updated
                );
                self.library_stale
                    .fetch_or(updated.is_some_and(|x| x > 0), Ordering::Relaxed);

                if let Ok(mut tasks) = self.pending_tasks.lock() {
                    if let Some(task) = tasks.pop_front() {
                        self.is_running = true;
                        task();
                    } else {
                        self.is_running = false;
                        self.library_stale.store(false, Ordering::Relaxed);
                        let _ = sender.output(BootstrapOutput::Completed);
                    }
                }

                if !self.is_running {
                    // Note: AtomicBool::swap returns previous value.
                    if self.stop.swap(false, Ordering::Relaxed) {
                        sender.input(BootstrapInput::Stopped);
                    }
                }
            }
            BootstrapInput::Stop => {
                info!("Stopping all background tasks");
                if self.is_running {
                    let _ = sender.output(BootstrapOutput::Stopping);
                    if let Ok(mut tasks) = self.pending_tasks.lock() {
                        tasks.clear();
                    }
                    self.stop.store(true, Ordering::Relaxed);
                } else {
                    sender.input(BootstrapInput::Stopped);
                }
            }
            other => {
                warn!("Ignoring {:?}! Please check this isn't a bug!", other);
            }
        };
    }

    fn add_task_photo_scan(&mut self) {
        let sender = self.photo_scan_task.sender().clone();
        self.enqueue(Box::new(move || sender.emit(PhotoScanTaskInput::Start)));
    }

    fn add_task_video_scan(&mut self) {
        let sender = self.video_scan_task.sender().clone();
        self.enqueue(Box::new(move || sender.emit(VideoScanTaskInput::Start)));
    }

    fn add_task_photo_enrich(&mut self) {
        let sender = self.photo_enrich_task.sender().clone();
        self.enqueue(Box::new(move || sender.emit(PhotoEnrichTaskInput::Start)));
    }

    fn add_task_video_enrich(&mut self) {
        let sender = self.video_enrich_task.sender().clone();
        self.enqueue(Box::new(move || sender.emit(VideoEnrichTaskInput::Start)));
    }

    fn add_task_photo_thumbnail(&mut self) {
        let sender = self.photo_thumbnail_task.sender().clone();
        self.enqueue(Box::new(move || {
            sender.emit(PhotoThumbnailTaskInput::Start)
        }));
    }

    fn add_task_video_thumbnail(&mut self) {
        let sender = self.video_thumbnail_task.sender().clone();
        self.enqueue(Box::new(move || {
            sender.emit(VideoThumbnailTaskInput::Start)
        }));
    }

    fn add_task_photo_clean(&mut self) {
        let sender = self.photo_clean_task.sender().clone();
        self.enqueue(Box::new(move || sender.emit(PhotoCleanTaskInput::Start)));
    }

    fn add_task_video_clean(&mut self) {
        let sender = self.video_clean_task.sender().clone();
        self.enqueue(Box::new(move || sender.emit(VideoCleanTaskInput::Start)));
    }

    fn add_task_photo_extract_motion(&mut self) {
        let sender = self.photo_extract_motion_task.sender().clone();
        self.enqueue(Box::new(move || {
            sender.emit(PhotoExtractMotionTaskInput::Start)
        }));
    }

    fn add_task_photo_detect_faces(&mut self) {
        let sender = self.photo_detect_faces_task.sender().clone();
        let mode = self.settings_state.read().face_detection_mode;
        match mode {
            FaceDetectionMode::Off => {}
            FaceDetectionMode::On => {
                self.enqueue(Box::new(move || {
                    sender.emit(PhotoDetectFacesTaskInput::DetectForAllPictures)
                }));
            }
        };
    }

    fn add_task_photo_detect_faces_for_one(&mut self, picture_id: PictureId) {
        let sender = self.photo_detect_faces_task.sender().clone();
        let mode = self.settings_state.read().face_detection_mode;
        match mode {
            FaceDetectionMode::Off => {}
            FaceDetectionMode::On => {
                self.enqueue(Box::new(move || {
                    sender.emit(PhotoDetectFacesTaskInput::DetectForOnePicture(picture_id))
                }));
            }
        };
    }

    fn add_task_photo_recognize_faces(&mut self) {
        let sender = self.photo_recognize_faces_task.sender().clone();
        let mode = self.settings_state.read().face_detection_mode;
        match mode {
            FaceDetectionMode::Off => {}
            FaceDetectionMode::On => {
                self.enqueue(Box::new(move || {
                    sender.emit(PhotoRecognizeFacesTaskInput::Start)
                }));
            }
        };
    }

    fn add_task_video_transcode(&mut self) {
        let sender = self.video_transcode_task.sender().clone();
        self.enqueue(Box::new(move || sender.emit(VideoTranscodeTaskInput::Start)));
    }

    fn add_task_load_library(&mut self, bootstrap_sender: Sender<BootstrapInput>) {
        let sender = self.load_library_task.sender().clone();
        let stale = self.library_stale.clone();
        let library_state = self.shared_state.clone();
        self.enqueue(Box::new(move || {
            if stale.load(Ordering::Relaxed) || library_state.read().is_empty() {
                info!("Library stale or empty so refreshing.");
                sender.emit(LoadLibraryTaskInput::Refresh);
            } else {
                bootstrap_sender.emit(BootstrapInput::TaskCompleted(TaskName::LoadLibrary, None));
            }
        }));
    }

    fn add_task_tidy(&mut self) {
        let sender = self.tidy_task.sender().clone();
        self.enqueue(Box::new(move || sender.emit(TidyTaskInput::Start)));
    }

    fn enqueue(&mut self, task: Box<dyn Fn() + Send + Sync>) {
        if let Ok(mut vec) = self.pending_tasks.lock() {
            vec.push_back(task);
        }
    }

    /// Run next task if no tasks running
    fn run_if_idle(&mut self) {
        if self.is_running {
            return;
        }
        if let Ok(mut tasks) = self.pending_tasks.lock() {
            if let Some(task) = tasks.pop_front() {
                info!("Running task right now");
                self.is_running = true;
                task();
            }
        }
    }
}

pub struct Bootstrap {
    settings_state: SettingsState,

    shared_state: SharedState,

    con: Arc<Mutex<database::Connection>>,

    progress_monitor: Arc<Reducer<ProgressMonitor>>,

    /// Background task runners. Only present after library path is set.
    controllers: Option<Controllers>,

    /// Current pictures base directory used by background tasks.
    pictures_base_dir: Option<PathBuf>,
}

impl Bootstrap {
    fn build_controllers(
        &mut self,
        library_sandbox_base_path: PathBuf,
        library_host_base_path: PathBuf,
        sender: &ComponentSender<Self>,
    ) -> anyhow::Result<Controllers> {
        let data_dir = glib::user_data_dir().join(APP_ID);
        let _ = std::fs::create_dir_all(&data_dir);

        let cache_dir = glib::user_cache_dir().join(APP_ID);
        let _ = std::fs::create_dir_all(&cache_dir);

        // WARN duplicate thumbnail path calculation in app.rs
        let thumbnail_dir = glib::user_cache_dir()
            .join(APP_ID) // Remove to use standard XDG thumbnail path
            .join("thumbnails");

        info!("Thumbnail directory is {:?}", thumbnail_dir);

        let thumbnailer = Thumbnailer::build(&thumbnail_dir);

        let photo_scanner = photo::Scanner::build(&library_sandbox_base_path)?;

        let photo_repo = photo::Repository::open(
            &library_sandbox_base_path,
            &library_host_base_path,
            &cache_dir,
            &data_dir,
            self.con.clone(),
        )?;

        let photo_thumbnailer = photo::PhotoThumbnailer::build(thumbnailer.clone())?;

        let video_scanner = video::Scanner::build(&library_sandbox_base_path)?;

        let video_repo = video::Repository::open(
            &library_sandbox_base_path,
            &library_host_base_path,
            &cache_dir,
            &data_dir,
            self.con.clone(),
        )?;

        let video_thumbnailer = video::VideoThumbnailer::build(thumbnailer.clone())?;

        let motion_photo_extractor = photo::MotionPhotoExtractor::build(&cache_dir)?;

        let visual_repo = visual::Repository::open(
            &library_sandbox_base_path,
            &library_host_base_path,
            &cache_dir,
            self.con.clone(),
        )?;

        let people_repo = people::Repository::open(
            &data_dir,
            self.con.clone())?;

        let stop = Arc::new(AtomicBool::new(false));

        let load_library_task = LoadLibraryTask::builder()
            .detach_worker((visual_repo.clone(), self.shared_state.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                LoadLibraryTaskOutput::Done => {
                    BootstrapInput::TaskCompleted(TaskName::LoadLibrary, None)
                }
            });

        let photo_scan_task = PhotoScanTask::builder()
            .detach_worker((photo_scanner.clone(), photo_repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                PhotoScanTaskOutput::Started => {
                    BootstrapInput::TaskStarted(TaskName::Scan(MediaType::Photo))
                }
                PhotoScanTaskOutput::Completed => {
                    BootstrapInput::TaskCompleted(TaskName::Scan(MediaType::Photo), None)
                }
            });

        let video_scan_task = VideoScanTask::builder()
            .detach_worker((video_scanner.clone(), video_repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                VideoScanTaskOutput::Started => {
                    BootstrapInput::TaskStarted(TaskName::Scan(MediaType::Video))
                }
                VideoScanTaskOutput::Completed => {
                    BootstrapInput::TaskCompleted(TaskName::Scan(MediaType::Video), None)
                }
            });

        let photo_enrich_task = PhotoEnrichTask::builder()
            .detach_worker((stop.clone(), photo_repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                PhotoEnrichTaskOutput::Started => {
                    BootstrapInput::TaskStarted(TaskName::Enrich(MediaType::Photo))
                }
                PhotoEnrichTaskOutput::Completed(count) => {
                    BootstrapInput::TaskCompleted(TaskName::Enrich(MediaType::Photo), Some(count))
                }
            });

        let video_enrich_task = VideoEnrichTask::builder()
            .detach_worker((
                stop.clone(),
                video_repo.clone(),
                self.progress_monitor.clone(),
            ))
            .forward(sender.input_sender(), |msg| match msg {
                VideoEnrichTaskOutput::Started => {
                    BootstrapInput::TaskStarted(TaskName::Enrich(MediaType::Video))
                }
                VideoEnrichTaskOutput::Completed(count) => {
                    BootstrapInput::TaskCompleted(TaskName::Enrich(MediaType::Video), Some(count))
                }
            });

        let photo_extract_motion_task = PhotoExtractMotionTask::builder()
            .detach_worker((
                stop.clone(),
                motion_photo_extractor,
                photo_repo.clone(),
                self.progress_monitor.clone(),
            ))
            .forward(sender.input_sender(), |msg| match msg {
                PhotoExtractMotionTaskOutput::Started => {
                    BootstrapInput::TaskStarted(TaskName::MotionPhoto)
                }
                PhotoExtractMotionTaskOutput::Completed(count) => {
                    BootstrapInput::TaskCompleted(TaskName::MotionPhoto, Some(count))
                }
            });

        let photo_thumbnail_task = PhotoThumbnailTask::builder()
            .detach_worker((
                stop.clone(),
                thumbnail_dir.clone(),
                photo_thumbnailer.clone(),
                photo_repo.clone(),
                self.progress_monitor.clone(),
            ))
            .forward(sender.input_sender(), |msg| match msg {
                PhotoThumbnailTaskOutput::Started => {
                    BootstrapInput::TaskStarted(TaskName::Thumbnail(MediaType::Photo))
                }
                PhotoThumbnailTaskOutput::Completed(count) => BootstrapInput::TaskCompleted(
                    TaskName::Thumbnail(MediaType::Photo),
                    Some(count),
                ),
            });

        let video_thumbnail_task = VideoThumbnailTask::builder()
            .detach_worker((
                stop.clone(),
                thumbnail_dir.clone(),
                video_thumbnailer.clone(),
                video_repo.clone(),
                self.progress_monitor.clone(),
            ))
            .forward(sender.input_sender(), |msg| match msg {
                VideoThumbnailTaskOutput::Started => {
                    BootstrapInput::TaskStarted(TaskName::Thumbnail(MediaType::Video))
                }
                VideoThumbnailTaskOutput::Completed(count) => BootstrapInput::TaskCompleted(
                    TaskName::Thumbnail(MediaType::Video),
                    Some(count),
                ),
            });

        let transcoder = video::Transcoder::new(&cache_dir);

        let video_transcode_task = VideoTranscodeTask::builder()
            .detach_worker((
                stop.clone(),
                self.shared_state.clone(),
                video_repo.clone(),
                transcoder,
                self.progress_monitor.clone(),
            ))
            .forward(sender.input_sender(), |msg| match msg {
                VideoTranscodeTaskOutput::Started => BootstrapInput::TaskStarted(TaskName::Transcode),
                VideoTranscodeTaskOutput::Completed => {
                    BootstrapInput::TaskCompleted(TaskName::Transcode, None)
                }
            });

        let photo_clean_task = PhotoCleanTask::builder()
            .detach_worker((stop.clone(), photo_repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                PhotoCleanTaskOutput::Started => {
                    BootstrapInput::TaskStarted(TaskName::Clean(MediaType::Photo))
                }
                PhotoCleanTaskOutput::Completed(count) => {
                    BootstrapInput::TaskCompleted(TaskName::Clean(MediaType::Photo), Some(count))
                }
            });

        let video_clean_task = VideoCleanTask::builder()
            .detach_worker((stop.clone(), video_repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                VideoCleanTaskOutput::Started => {
                    BootstrapInput::TaskStarted(TaskName::Clean(MediaType::Video))
                }
                VideoCleanTaskOutput::Completed(count) => {
                    BootstrapInput::TaskCompleted(TaskName::Clean(MediaType::Video), Some(count))
                }
            });

        let photo_detect_faces_task = PhotoDetectFacesTask::builder()
            .detach_worker((
                stop.clone(),
                data_dir,
                thumbnailer,
                photo_repo.clone(),
                people_repo.clone(),
                self.progress_monitor.clone(),
            ))
            .forward(sender.input_sender(), |msg| match msg {
                PhotoDetectFacesTaskOutput::Started => {
                    BootstrapInput::TaskStarted(TaskName::DetectFaces)
                }
                PhotoDetectFacesTaskOutput::Completed => {
                    BootstrapInput::TaskCompleted(TaskName::DetectFaces, None)
                }
            });

        let photo_recognize_faces_task = PhotoRecognizeFacesTask::builder()
            .detach_worker((
                stop.clone(),
                cache_dir.clone(),
                people_repo.clone(),
                self.progress_monitor.clone(),
            ))
            .forward(sender.input_sender(), |msg| match msg {
                PhotoRecognizeFacesTaskOutput::Started => {
                    BootstrapInput::TaskStarted(TaskName::RecognizeFaces)
                }
                PhotoRecognizeFacesTaskOutput::Completed => {
                    BootstrapInput::TaskCompleted(TaskName::RecognizeFaces, None)
                }
            });

        let tidy_task = TidyTask::builder()
            .detach_worker(stop.clone())
            .forward(sender.input_sender(), |msg| match msg {
                TidyTaskOutput::Started => {
                    BootstrapInput::TaskStarted(TaskName::Tidy)
                }
                TidyTaskOutput::Completed => {
                    BootstrapInput::TaskCompleted(TaskName::Tidy, None)
                }
            });

        let mut controllers = Controllers {
            stop,
            started_at: None,
            shared_state: self.shared_state.clone(),
            settings_state: self.settings_state.clone(),
            load_library_task: Arc::new(load_library_task),
            photo_scan_task: Arc::new(photo_scan_task),
            video_scan_task: Arc::new(video_scan_task),
            photo_enrich_task: Arc::new(photo_enrich_task),
            video_enrich_task: Arc::new(video_enrich_task),
            photo_extract_motion_task: Arc::new(photo_extract_motion_task),
            photo_clean_task: Arc::new(photo_clean_task),
            video_clean_task: Arc::new(video_clean_task),
            photo_thumbnail_task: Arc::new(photo_thumbnail_task),
            video_thumbnail_task: Arc::new(video_thumbnail_task),
            photo_detect_faces_task: Arc::new(photo_detect_faces_task),
            photo_recognize_faces_task: Arc::new(photo_recognize_faces_task),
            video_transcode_task: Arc::new(video_transcode_task),
            tidy_task: Arc::new(tidy_task),
            pending_tasks: Arc::new(Mutex::new(VecDeque::new())),
            is_running: false,
            library_stale: Arc::new(AtomicBool::new(true)),
        };

        // Tasks will execute in the order added.

        // Initial library load to reduce time from starting app and seeing a photo grid
        controllers.add_task_load_library(sender.input_sender().clone());
        controllers.add_task_photo_scan();
        controllers.add_task_video_scan();
        controllers.add_task_photo_enrich();
        controllers.add_task_video_enrich();

        // If loaded library is currently empty, then refresh now that the photo and video scans
        // are complete. Note: should do this after enriching because otherwise Fotema won't
        // have processed the orientation metadata and will display pictures incorrectly.
        controllers.add_task_load_library(sender.input_sender().clone());

        controllers.add_task_photo_thumbnail();
        controllers.add_task_video_thumbnail();
        controllers.add_task_photo_clean();
        controllers.add_task_video_clean();
        controllers.add_task_photo_extract_motion();
        controllers.add_task_photo_detect_faces();
        controllers.add_task_photo_recognize_faces();

        controllers.add_task_tidy();

        // This is the last background task to complete. Refresh library if there
        // has been a visible change to the library state.
        controllers.add_task_load_library(sender.input_sender().clone());

        Ok(controllers)
    }
}

impl Worker for Bootstrap {
    type Init = (
        Arc<Mutex<database::Connection>>,
        SharedState,
        SettingsState,
        Arc<Reducer<ProgressMonitor>>,
    );
    type Input = BootstrapInput;
    type Output = BootstrapOutput;

    fn init(
        (con, shared_state, settings_state, progress_monitor): Self::Init,
        sender: ComponentSender<Self>,
    ) -> Self {
        settings_state.subscribe(sender.input_sender(), |settings| {
            BootstrapInput::SettingsUpdated(settings.clone())
        });

        Self {
            shared_state,
            settings_state,
            progress_monitor,
            con,
            controllers: None,
            pictures_base_dir: None,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        // This match block coordinates the background tasks launched immediately after
        // the app starts up.
        match msg {
            BootstrapInput::Configure(pictures_base_dir, pictures_base_dir_host_path) => {
                info!(
                    "Configuring with pictures base directory: {:?}",
                    pictures_base_dir
                );

                match self.build_controllers(
                    pictures_base_dir.clone(),
                    pictures_base_dir_host_path,
                    &sender,
                ) {
                    Ok(controllers) => {
                        self.pictures_base_dir = Some(pictures_base_dir);
                        self.controllers = Some(controllers);
                        sender.input(BootstrapInput::Start);
                    }
                    Err(e) => {
                        error!("Failed to build background ask controllers: {:?}", e);
                    }
                }
            }
            BootstrapInput::SettingsUpdated(settings) => {
                info!("Settings updated.");
                // Only stop, reconfigure, and restart tasks if pictures dir changes.
                if self
                    .pictures_base_dir
                    .as_ref()
                    .is_some_and(|dir| *dir != settings.pictures_base_dir)
                {
                    // If running, then shutdown running and queued tasks, and then reconfigure.
                    // Otherwise simply reconfigure with new path.
                    if self
                        .controllers
                        .as_ref()
                        .is_some_and(|controllers| controllers.is_running)
                    {
                        self.pictures_base_dir = None;
                        sender.input(BootstrapInput::Stop);
                    } else {
                        self.controllers = None;
                        sender.input(BootstrapInput::Configure(
                            settings.pictures_base_dir,
                            settings.pictures_base_dir_host_path,
                        ));
                    }
                }
            }
            BootstrapInput::Stopped if self.pictures_base_dir.is_none() => {
                // If stopped and no pictures base dir, then background tasks were
                // shutdown in response to the user changing the pictures base directory.
                // Now that tasks are shutdown, it is safe to reconfigure with
                // the new directory.
                let settings = self.settings_state.read();
                sender.input(BootstrapInput::Configure(
                    settings.pictures_base_dir.clone(),
                    settings.pictures_base_dir_host_path.clone(),
                ));
            }
            BootstrapInput::TaskCompleted(TaskName::LoadLibrary, _)
                if self.controllers.is_some() =>
            {
                info!(
                    "Forwarding {:?} to controllers and marking library as fresh.",
                    msg
                );
                if let Some(ref mut controllers) = self.controllers {
                    controllers.library_stale.store(false, Ordering::Relaxed);
                    controllers.update(msg, sender);
                }
            }
            msg if self.controllers.is_some() => {
                info!("Forwarding {:?} to controllers.", msg);
                if let Some(ref mut controllers) = self.controllers {
                    controllers.update(msg, sender);
                }
            }
            _ => {
                info!("Ignore {:?} because bootstrap is unconfigured.", msg);
            }
        };
    }
}
