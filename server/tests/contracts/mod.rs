//! Focused integration contracts share fixtures but never mutable test state.

/// Albums behavior.
mod albums;
/// Auth behavior.
mod auth;
/// Bootstrap behavior.
mod bootstrap;
/// Delivery behavior.
mod delivery;
/// Shared image/video lifecycle and original-byte preservation.
mod images;
/// Migrations behavior.
mod migrations;
/// Restart-safe object relocation and legacy layout upgrades.
mod objects;
/// Openapi behavior.
mod openapi;
/// Privacy behavior.
mod privacy;
/// Security behavior.
mod security;
/// Storage behavior.
mod storage;
/// Uploads behavior.
mod uploads;
