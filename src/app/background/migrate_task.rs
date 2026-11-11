// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Result;
use relm4::Worker;
use relm4::prelude::*;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tracing::{error, info};

use fotema_core::people::migrate::Migrate;
use fotema_core::thumbnailify::Thumbnailer;

#[derive(Debug)]
pub enum MigrateTaskInput {
    Start,
}

#[derive(Debug)]
pub enum MigrateTaskOutput {
    Started,
    Completed,
}

pub struct MigrateTask {
    // Stop flag
    stop: Arc<AtomicBool>,

    migrate: Migrate,

    thumbnailer: Thumbnailer,
}

impl MigrateTask {
    fn migrate(&mut self, sender: &ComponentSender<MigrateTask>) -> Result<()> {
        let _ = sender.output(MigrateTaskOutput::Started);

        let _ = self.migrate.migrate().map_err(|e| {
            error!("Failed migration: {:?}", e);
            e
        });

        self.migrate_thumbnail_cache();

        let _ = sender.output(MigrateTaskOutput::Completed);

        Ok(())
    }

    /// Replace each PNG thumbnail of an older build with a JPEG thumbnail. Then
    /// delete the PNG file. A PNG thumbnail is 5 to 10 times larger. Thus a cache
    /// from an older build uses too much disk space until this task migrates it.
    ///
    /// This task runs before the thumbnail tasks. Thus those tasks find a cache in
    /// the current format. You can stop the application during the migration. The
    /// task converts each thumbnail independently. It deletes a PNG file only
    /// after the JPEG file is in position. The next start continues the work.
    fn migrate_thumbnail_cache(&self) {
        let stats = self.thumbnailer.migrate_legacy_png_thumbnails();

        if stats.total_removed() > 0 {
            info!(
                "Migrated {} legacy PNG thumbnails to JPEG ({} superseded), freeing {} KiB",
                stats.converted,
                stats.superseded,
                stats.bytes_freed / 1024
            );
        }
    }
}

impl Worker for MigrateTask {
    type Init = (Arc<AtomicBool>, Migrate, Thumbnailer);
    type Input = MigrateTaskInput;
    type Output = MigrateTaskOutput;

    fn init((stop, migrate, thumbnailer): Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self {
            stop,
            migrate,
            thumbnailer,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        if self.stop.load(Ordering::Relaxed) {
            let _ = sender.output(MigrateTaskOutput::Completed);
            return;
        }

        match msg {
            MigrateTaskInput::Start => {
                info!("Migrating...");

                if let Err(e) = self.migrate(&sender) {
                    error!("Failed to migrate: {}", e);
                }
            }
        };
    }
}
