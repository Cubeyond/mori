// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! HTTP and S3 orchestration regression tests using isolated protocol fixtures.

/// SQLite WAL initialization and embedded migrations.
pub mod db;
/// Checksummed S3 operations and bounded AWS clients.
pub mod storage;

/// Encrypted configuration and runtime storage-client management.
pub mod storage_settings;
