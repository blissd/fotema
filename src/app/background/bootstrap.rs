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
use fotema_core::machine_learning;
use fotema_core::people;
use fotema_core::PictureId;

use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::collections::VecDeque;

use tracing::info;

use super::{
    load_library::{LoadLibrary, LoadLibraryInput},

    photo_clean::{PhotoClean, PhotoCleanInput, PhotoCleanOutput},
    photo_detect_faces::{PhotoDetectFaces, PhotoDetectFacesInput, PhotoDetectFacesOutput},
    photo_enrich::{PhotoEnrich, PhotoEnrichInput, PhotoEnrichOutput},
    photo_recognize_faces::{PhotoRecognizeFaces, PhotoRecognizeFacesInput, PhotoRecognizeFacesOutput},
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
    DetectFaces,
    RecognizeFaces,
}

#[derive(Debug)]
pub enum BootstrapInput {

    /// Start the initial background processes for setting up Fotema.
    Start,

    /// Queue task for scanning picture for more faces.
    ScanForFaces(PictureId),

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

}


pub struct Bootstrap {
    started_at: Option<Instant>,

    /// Whether a background task has updated some library state and the library should be reloaded.
    library_stale: bool,

    load_library: Arc<WorkerController<LoadLibrary>>,

    photo_scan: Arc<WorkerController<PhotoScan>>,
    video_scan: Arc<WorkerController<VideoScan>>,

    photo_enrich: Arc<WorkerController<PhotoEnrich>>,
    video_enrich: Arc<WorkerController<VideoEnrich>>,

    photo_clean: Arc<WorkerController<PhotoClean>>,
    video_clean: Arc<WorkerController<VideoClean>>,

    photo_thumbnail: Arc<WorkerController<PhotoThumbnail>>,
    video_thumbnail: Arc<WorkerController<VideoThumbnail>>,

    photo_extract_motion: Arc<WorkerController<PhotoExtractMotion>>,

    photo_detect_faces: Arc<WorkerController<PhotoDetectFaces>>,
    photo_recognize_faces: Arc<WorkerController<PhotoRecognizeFaces>>,

    /// Pending ordered tasks to process
    /// Wow... figuring out a type signature that would compile was a nightmare.
    pending_tasks: Arc<Mutex<VecDeque<Box<dyn Fn() + Send + Sync>>>>,

    // Is a task currently running?
    is_running: bool,
}

impl Bootstrap {
    fn add_task_photo_scan(&mut self)  {
        let sender = self.photo_scan.sender().clone();
        self.enqueue(Box::new(move || sender.emit(PhotoScanInput::Start)));
    }

    fn add_task_video_scan(&mut self) {
        let sender = self.video_scan.sender().clone();
        self.enqueue(Box::new(move || sender.emit(VideoScanInput::Start)));
    }

    fn add_task_photo_enrich(&mut self) {
        let sender = self.photo_enrich.sender().clone();
        self.enqueue(Box::new(move || sender.emit(PhotoEnrichInput::Start)));
    }

    fn add_task_video_enrich(&mut self) {
        let sender = self.video_enrich.sender().clone();
        self.enqueue(Box::new(move || sender.emit(VideoEnrichInput::Start)));
    }

    fn add_task_photo_thumbnail(&mut self) {
        let sender = self.photo_thumbnail.sender().clone();
        self.enqueue(Box::new(move || sender.emit(PhotoThumbnailInput::Start)));
    }

    fn add_task_video_thumbnail(&mut self) {
        let sender = self.video_thumbnail.sender().clone();
        self.enqueue(Box::new(move || sender.emit(VideoThumbnailInput::Start)));
    }

    fn add_task_photo_clean(&mut self) {
        let sender = self.photo_clean.sender().clone();
        self.enqueue(Box::new(move || sender.emit(PhotoCleanInput::Start)));
    }

    fn add_task_video_clean(&mut self) {
        let sender = self.video_clean.sender().clone();
        self.enqueue(Box::new(move || sender.emit(VideoCleanInput::Start)));
    }

    fn add_task_photo_extract_motion(&mut self) {
        let sender = self.photo_extract_motion.sender().clone();
        self.enqueue(Box::new(move || sender.emit(PhotoExtractMotionInput::Start)));
    }

    fn add_task_photo_detect_faces(&mut self) {
        let sender = self.photo_detect_faces.sender().clone();
        self.enqueue(Box::new(move || sender.emit(PhotoDetectFacesInput::DetectForAllPictures)));
    }

    fn add_and_run_task_photo_detect_faces_for_one(&mut self, picture_id: PictureId) {
        let sender = self.photo_detect_faces.sender().clone();
        self.enqueue_or_run(Box::new(move || sender.emit(PhotoDetectFacesInput::DetectForOnePicture(picture_id))));
    }

    fn add_task_photo_recognize_faces(&mut self) {
        let sender = self.photo_recognize_faces.sender().clone();
        self.enqueue(Box::new(move || sender.emit(PhotoRecognizeFacesInput::Start)));
    }

    fn enqueue(&mut self, task: Box<dyn Fn() + Send + Sync>) {
        if let Ok(mut vec) = self.pending_tasks.lock() {
            vec.push_back(task);
        }
    }

    /// Enqueues task, or runs task immediately if no other tasks pending
    fn enqueue_or_run(&mut self, task: Box<dyn Fn() + Send + Sync>) {
        if let Ok(mut vec) = self.pending_tasks.lock() {
            if self.is_running {
                info!("Enqueing new task");
                vec.push_back(task);
            } else {
                info!("Running new task right now");
                self.is_running = true;
                task();
            }
        }
    }
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
        ).unwrap();

        let people_repo = people::Repository::open(
            &pic_base_dir,
            &data_dir,
            con.clone(),
        )
        .unwrap();

        let face_extractor = machine_learning::face_extractor::FaceExtractor::build(&data_dir).unwrap();

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
                PhotoCleanOutput::Completed(count) => BootstrapInput::TaskCompleted(TaskName::Clean(MediaType::Photo), Some(count)),
            });

        let video_clean = VideoClean::builder()
            .detach_worker(video_repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                VideoCleanOutput::Started => BootstrapInput::TaskStarted(TaskName::Clean(MediaType::Video)),
                VideoCleanOutput::Completed(count) => BootstrapInput::TaskCompleted(TaskName::Clean(MediaType::Video), Some(count)),
            });

        let photo_detect_faces = PhotoDetectFaces::builder()
            .detach_worker((face_extractor, people_repo.clone(), progress_monitor.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                PhotoDetectFacesOutput::Started => BootstrapInput::TaskStarted(TaskName::DetectFaces),
                PhotoDetectFacesOutput::Completed(count) => BootstrapInput::TaskCompleted(TaskName::DetectFaces, Some(count)),
            });

        let photo_recognize_faces = PhotoRecognizeFaces::builder()
            .detach_worker((people_repo.clone(), progress_monitor.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                PhotoRecognizeFacesOutput::Started => BootstrapInput::TaskStarted(TaskName::RecognizeFaces),
                PhotoRecognizeFacesOutput::Completed(count) => BootstrapInput::TaskCompleted(TaskName::RecognizeFaces, Some(count)),
            });

        let mut bootstrap = Self {
            started_at: None,
            library_stale: false,
            load_library: Arc::new(load_library),
            photo_scan: Arc::new(photo_scan),
            video_scan: Arc::new(video_scan),
            photo_enrich: Arc::new(photo_enrich),
            video_enrich:Arc::new(video_enrich),
            photo_extract_motion: Arc::new(photo_extract_motion),
            photo_clean: Arc::new(photo_clean),
            video_clean: Arc::new(video_clean),
            photo_thumbnail: Arc::new(photo_thumbnail),
            video_thumbnail: Arc::new(video_thumbnail),
            photo_detect_faces: Arc::new(photo_detect_faces),
            photo_recognize_faces: Arc::new(photo_recognize_faces),
            pending_tasks: Arc::new(Mutex::new(VecDeque::new())),
            is_running: false,
        };

        // Tasks will execute in the order added.
        bootstrap.add_task_photo_scan();
        bootstrap.add_task_video_scan();
        bootstrap.add_task_photo_enrich();
        bootstrap.add_task_video_enrich();
        bootstrap.add_task_photo_thumbnail();
        bootstrap.add_task_video_thumbnail();
        bootstrap.add_task_photo_clean();
        bootstrap.add_task_video_clean();
        bootstrap.add_task_photo_extract_motion();
        bootstrap.add_task_photo_detect_faces();
        bootstrap.add_task_photo_recognize_faces();

        bootstrap
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

                if let Ok(mut tasks) = self.pending_tasks.lock() {
                    if let Some(task) = tasks.pop_front() {
                        self.is_running = true;
                        task();
                    } else {
                        self.is_running = false;
                        let _ = sender.output(BootstrapOutput::Completed);
                    }
                }
            },
            BootstrapInput::ScanForFaces(picture_id) => {
                info!("Queueing task to scan picture {} for faces", picture_id);
                self.add_and_run_task_photo_detect_faces_for_one(picture_id);
            },
            BootstrapInput::TaskStarted(task_name) => {
                info!("Task started: {:?}", task_name);
                let _  = sender.output(BootstrapOutput::TaskStarted(task_name));
            },
            BootstrapInput::TaskCompleted(task_name, updated) => {
                info!("Task completed: {:?}. Items updated? {:?}", task_name, updated);
                self.library_stale = self.library_stale || updated.is_some_and(|x| x > 0);

                if let Ok(mut tasks) = self.pending_tasks.lock() {
                    if let Some(task) = tasks.pop_front() {
                        self.is_running = true;
                        task();
                    } else {
                        // This is the last background task to complete. Refresh library if there
                        // has been a visible change to the library state.
                        if self.library_stale {
                            info!("Refreshing library final task completion.");
                            self.load_library.emit(LoadLibraryInput::Refresh);
                        }
                        self.library_stale = false;
                        self.is_running = false;
                        let _ = sender.output(BootstrapOutput::Completed);
                    }
                }
            },
        };
    }
}
