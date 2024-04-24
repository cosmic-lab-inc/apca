// Copyright (C) 2022-2024 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::DateTime;
use chrono::Utc;

use serde::Deserialize;
use serde::Serialize;
use serde_urlencoded::to_string as to_query;

use crate::data::v2::Feed;
use crate::data::DATA_BASE_URL;
use crate::data::v2::prefix::MarketPrefix;
use crate::util::vec_from_str;
use crate::Str;

/// A quote as returned by the /v2/stocks/{symbol}/quotes endpoint.
pub use super::last_quotes::Quote;


/// A collection of quotes as returned by the API. This is one page of
/// quotes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Quotes {
  /// The list of returned quotes.
  #[serde(rename = "quotes", deserialize_with = "vec_from_str")]
  pub quotes: Vec<Quote>,
  /// The symbol the quotes correspond to.
  #[serde(rename = "symbol")]
  pub symbol: String,
  /// The token to provide to a request to get the next page of quotes
  /// for this request.
  #[serde(rename = "next_page_token")]
  pub next_page_token: Option<String>,
  /// The type is non-exhaustive and open to extension.
  #[doc(hidden)]
  #[serde(skip)]
  pub _non_exhaustive: (),
}


/// A helper for initializing [`ListReq`] objects.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ListReqInit {
  /// See `ListReq::limit`.
  pub limit: Option<usize>,
  /// See `ListReq::feed`.
  pub feed: Option<Feed>,
  /// See `ListReq::page_token`.
  pub page_token: Option<String>,
  /// The type is non-exhaustive and open to extension.
  #[doc(hidden)]
  pub _non_exhaustive: (),
}

impl ListReqInit {
  /// Create a [`ListReq`] from a `ListReqInit`.
  #[inline]
  pub fn init<S>(self, symbol: S, prefix: MarketPrefix, start: DateTime<Utc>, end: DateTime<Utc>) -> ListReq
  where
    S: Into<String>,
  {
    ListReq {
      symbol: symbol.into(),
      prefix,
      start,
      end,
      limit: self.limit,
      feed: self.feed,
      page_token: self.page_token,
      _non_exhaustive: (),
    }
  }
}


/// A GET request to be made to the /v2/stocks/{symbol}/quotes endpoint.
// TODO: Not all fields are hooked up.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ListReq {
  /// The symbol to retrieve quotes for.
  #[serde(skip)]
  pub symbol: String,
  /// The path prefix based on the market (e.g. stocks or crypto)
  /// Crypto = /v1beta3/crypto/us/
  /// Stocks = /v2/stocks/
  pub prefix: MarketPrefix,
  /// Filter data equal to or after this time in RFC-3339 format.
  /// Defaults to the current day in CT.
  #[serde(rename = "start")]
  pub start: DateTime<Utc>,
  /// Filter data equal to or before this time in RFC-3339 format.
  /// Default value is now.
  #[serde(rename = "end")]
  pub end: DateTime<Utc>,
  /// Number of quotes to return. Must be in range 1-10000, defaults to
  /// 1000.
  #[serde(rename = "limit")]
  pub limit: Option<usize>,
  /// The data feed to use.
  #[serde(rename = "feed")]
  pub feed: Option<Feed>,
  /// Pagination token to continue from.
  #[serde(rename = "page_token")]
  pub page_token: Option<String>,
  /// The type is non-exhaustive and open to extension.
  #[doc(hidden)]
  #[serde(skip)]
  pub _non_exhaustive: (),
}


Endpoint! {
  /// The representation of a GET request to the
  /// /v2/stocks/{symbol}/quotes endpoint.
  pub List(ListReq),
  Ok => Quotes, [
    /// The quote information was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => ListError, [
    /// Some of the provided data was invalid or not found.
    /* 400 */ BAD_REQUEST => InvalidInput,
  ]

  fn base_url() -> Option<Str> {
    Some(DATA_BASE_URL.into())
  }

  #[inline]
  fn path(input: &Self::Input) -> Str {
    format!("{}{}/quotes", input.prefix, input.symbol).into()
  }

  fn query(input: &Self::Input) -> Result<Option<Str>, Self::ConversionError> {
    Ok(Some(to_query(input)?.into()))
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::str::FromStr as _;

  use num_decimal::Num;

  use test_log::test;

  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::RequestError;


  /// Check that we can retrieve quotes for a specific time frame.
  #[test(tokio::test)]
  async fn request_quotes() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let start = DateTime::from_str("2022-01-04T13:35:59Z").unwrap();
    let end = DateTime::from_str("2022-01-04T13:36:00Z").unwrap();
    let request = ListReqInit::default().init("SPY", MarketPrefix::Stocks, start, end);
    let quotes = client.issue::<List>(&request).await.unwrap();

    assert_eq!(&quotes.symbol, "SPY");

    for quote in quotes.quotes {
      assert!(quote.time >= start, "{}", quote.time);
      assert!(quote.time <= end, "{}", quote.time);
      assert_ne!(quote.ask_price, Num::from(0));
      assert_ne!(quote.bid_price, Num::from(0));
      assert_ne!(quote.ask_size, 0);
      assert_ne!(quote.bid_size, 0);
    }
  }

  /// Verify that we can specify the SIP feed as the data source to use.
  #[test(tokio::test)]
  async fn sip_feed() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let start = DateTime::from_str("2022-01-04T13:35:59Z").unwrap();
    let end = DateTime::from_str("2022-01-04T13:36:00Z").unwrap();
    let request = ListReqInit::default().init("SPY", MarketPrefix::Stocks, start, end);
    let result = client.issue::<List>(&request).await;
    // Unfortunately we can't really know whether the user has the
    // unlimited plan and can access the SIP feed. So really all we can
    // do here is accept both possible outcomes.
    match result {
      Ok(_) | Err(RequestError::Endpoint(ListError::NotPermitted(_))) => (),
      err => panic!("Received unexpected error: {err:?}"),
    }
  }

  /// Verify that we error out as expected when attempting to retrieve
  /// the quotes for an invalid symbol.
  #[test(tokio::test)]
  async fn invalid_symbol() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let start = DateTime::from_str("2022-01-04T13:35:59Z").unwrap();
    let end = DateTime::from_str("2022-01-04T13:36:00Z").unwrap();
    let request = ListReqInit::default().init("ABC123", MarketPrefix::Stocks, start, end);
    let err = client.issue::<List>(&request).await.unwrap_err();
    match err {
      RequestError::Endpoint(ListError::InvalidInput(Ok(_))) => (),
      _ => panic!("Received unexpected error: {err:?}"),
    };
  }

  /// Check that we fail as expected when an invalid page token is
  /// specified.
  #[test(tokio::test)]
  async fn invalid_page_token() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let start = DateTime::from_str("2022-01-04T13:35:59Z").unwrap();
    let end = DateTime::from_str("2022-01-04T13:36:00Z").unwrap();
    let request = ListReqInit {
      page_token: Some("123456789abcdefghi".to_string()),
      ..Default::default()
    }
    .init("SPY", MarketPrefix::Stocks, start, end);

    let err = client.issue::<List>(&request).await.unwrap_err();
    match err {
      RequestError::Endpoint(ListError::InvalidInput(_)) => (),
      _ => panic!("Received unexpected error: {err:?}"),
    };
  }

  /// Check that we can page quotes as expected.
  #[test(tokio::test)]
  async fn page_quotes() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let start = DateTime::from_str("2022-01-04T13:35:00Z").unwrap();
    let end = DateTime::from_str("2022-01-04T13:36:00Z").unwrap();
    let mut request = ListReqInit {
      limit: Some(2),
      ..Default::default()
    }
    .init("SPY", MarketPrefix::Stocks, start, end);

    let mut last_quotes = None;
    // We assume that there are at least three pages of two quotes.
    for _ in 0..3 {
      let quotes = client.issue::<List>(&request).await.unwrap();
      assert_ne!(Some(quotes.clone()), last_quotes);

      request.page_token = quotes.next_page_token.clone();
      last_quotes = Some(quotes);
    }
  }
}
