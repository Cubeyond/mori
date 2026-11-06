// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

//! Bounded shared pagination parameters for collection endpoints.

use serde::Deserialize;
use utoipa::IntoParams;

/// Pagination inputs normalized to bounded query limits.
#[derive(Default, Deserialize, IntoParams)]
#[into_params(parameter_in=Query)]
pub struct Page {
    /// Optional pagination offset clamped to the supported range.
    pub offset: Option<i64>,
    /// Optional page size clamped to the configured response budget.
    pub limit: Option<usize>,
    /// Optional literal display-name search, limited to 200 characters.
    pub q: Option<String>,
}
impl Page {
    /// Return a bounded effective page size.
    #[must_use]
    pub fn limit(&self) -> usize {
        self.limit.unwrap_or(30).clamp(1, 60)
    }
    /// Return a bounded nonnegative pagination offset.
    #[must_use]
    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).clamp(0, 1_000_000)
    }
}
