// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::*;
use rusqlite::Connection;
use std::path;

// Embed migration SQL in executable.
refinery::embed_migrations!("migrations");

pub fn setup(database_path: &path::Path) -> Result<Connection> {
    let mut con = Connection::open(database_path)?;
    migrations::runner().run(&mut con)?;
    Ok(con)
}

// for testing
pub fn setup_in_memory() -> Result<Connection> {
    let mut con = Connection::open_in_memory()?;
    migrations::runner().run(&mut con)?;
    Ok(con)
}
