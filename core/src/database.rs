// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::Error::RepositoryError;
use crate::Result;
use rusqlite::Connection;
use std::path;

// Embed migration SQL in executable.
refinery::embed_migrations!("migrations");

pub fn setup(database_path: &path::Path) -> Result<Connection> {
    let mut con = Connection::open(database_path).map_err(|e| RepositoryError(e.to_string()))?;
    migrations::runner()
        .run(&mut con)
        .map_err(|e| RepositoryError(e.to_string()))?;
    Ok(con)
}

// for testing
pub fn setup_in_memory() -> Result<Connection> {
    let mut con = Connection::open_in_memory().map_err(|e| RepositoryError(e.to_string()))?;
    migrations::runner()
        .run(&mut con)
        .map_err(|e| RepositoryError(e.to_string()))?;
    Ok(con)
}
