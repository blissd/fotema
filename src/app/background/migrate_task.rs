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
}

impl MigrateTask {
    fn migrate(&mut self, sender: &ComponentSender<MigrateTask>) -> Result<()> {

        let _= sender.output(MigrateTaskOutput::Started);

        let _ = self.migrate.migrate()
            .map_err(|e| {
                error!("Failed migration: {:?}", e);
                e
            });

        let _= sender.output(MigrateTaskOutput::Completed);

        Ok(())
    }
}

impl Worker for MigrateTask {
    type Init = (Arc<AtomicBool>, Migrate);
    type Input = MigrateTaskInput;
    type Output = MigrateTaskOutput;

    fn init((stop, migrate): Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { stop, migrate }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        if self.stop.load(Ordering::Relaxed) {
            let _= sender.output(MigrateTaskOutput::Completed);
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
