// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt::Display;

/// Database ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersonId(i64);

impl PersonId {
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    /// FIXME replace this with a To/From SQL implementation.
    pub fn id(&self) -> i64 {
        self.0
    }
}

impl Display for PersonId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Person {
    pub person_id: PersonId,
    pub name: String,
}
