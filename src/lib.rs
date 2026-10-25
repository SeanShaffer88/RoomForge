//! RoomForge library: the data model, furniture catalog, geometry,
//! 3D projection, exports and file storage. All decision logic lives
//! here so it can be tested without a GUI or a Windows machine.
//! The GUI in `main.rs` / `app.rs` only renders state and collects intent.

pub mod catalog;
pub mod export;
pub mod furniture;
pub mod geometry;
pub mod io;
pub mod model;
pub mod model3d;
pub mod render;
pub mod units;
pub mod view3d;
pub mod walk;
