//! Game-side projections of `Inventory` for the UI. None of these
//! functions talk to freenet or do any I/O — they're pure
//! computations the renderer uses to convert the delegate's
//! authoritative state into something to draw.

pub mod derived;
