// Copyright (C) 2021-2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

mod feed;
mod unfold;

/// Definitions for retrieval of market data bars.
pub mod bars;
/// Functionality for retrieval of most recent quotes.
pub mod last_quotes;
/// Functionality for retrieving historic quotes.
pub mod quotes;
/// Definitions for real-time streaming of market data.
pub mod stream;
/// Definitions for retrieval of market data trades.
pub mod trades;
/// Definitions for market path prefixes
pub mod prefix;

pub use feed::Feed;

