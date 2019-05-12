// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use hyper::Body;
use hyper::http::Error as HttpError;
use hyper::http::request::Builder;
use hyper::Method;
use hyper::Request;

use serde::de::DeserializeOwned;
use serde_json::Error as JsonError;
use serde_json::from_slice;

use url::Url;

use crate::api::HDR_KEY_ID;
use crate::api::HDR_SECRET;
use crate::Str;


/// An error type used by the `Endpoint` trait.
#[derive(Debug)]
pub enum EndpointError {
  /// An HTTP related error.
  Http(HttpError),
  /// A JSON conversion error.
  Json(JsonError),
}

impl From<HttpError> for EndpointError {
  fn from(e: HttpError) -> Self {
    EndpointError::Http(e)
  }
}

impl From<JsonError> for EndpointError {
  fn from(e: JsonError) -> Self {
    EndpointError::Json(e)
  }
}


/// A trait describing an HTTP endpoint.
///
/// An endpoint for our intents and purposes is basically a path and an
/// HTTP request method (e.g., GET or POST). The path will be combined
/// with an "authority" (scheme, host, and port) into a full URL. Query
/// parameters are supported as well.
/// An endpoint is used by the `Trader` who invokes the various methods.
pub trait Endpoint {
  /// The type of data being passed in as part of a request to this
  /// endpoint.
  type Input;
  /// The type of data being returned in the response from this
  /// endpoint.
  type Output;
  /// The type of error this endpoint can report.
  type Error;

  /// Retrieve the HTTP method to use.
  ///
  /// The default method being used is GET.
  fn method() -> Method {
    Method::GET
  }

  /// Inquire the path the request should go to.
  fn path(input: &Self::Input) -> Str;

  /// Inquire the query the request should use.
  ///
  /// By default no query is emitted.
  #[allow(unused)]
  fn query(input: &Self::Input) -> Option<Str> {
    None
  }

  /// Retrieve the request's body.
  ///
  /// By default this method creates an empty body.
  #[allow(unused)]
  fn body(input: &Self::Input) -> Result<Body, JsonError> {
    Ok(Body::empty())
  }

  /// Create a `Request` to the endpoint.
  ///
  /// Typically the default implementation is just fine.
  fn request(
    api_base: &Url,
    key_id: &[u8],
    secret: &[u8],
    input: &Self::Input,
  ) -> Result<Request<Body>, EndpointError> {
    let mut url = api_base.clone();
    url.set_path(&Self::path(&input));
    url.set_query(Self::query(&input).as_ref().map(AsRef::as_ref));

    Builder::new()
      .method(Self::method())
      .uri(url.as_str())
      // Add required authentication information.
      .header(HDR_KEY_ID, key_id)
      .header(HDR_SECRET, secret)
      .body(Self::body(input)?)
      .map_err(EndpointError::from)
  }

  /// Parse the body into the final result.
  ///
  /// By default this method directly parses the body as JSON.
  fn parse(body: &[u8]) -> Result<Self::Output, Self::Error>
  where
    Self::Output: DeserializeOwned,
    Self::Error: From<JsonError>,
  {
    from_slice::<Self::Output>(body).map_err(Self::Error::from)
  }
}


/// A result type used solely for the purpose of communicating
/// the result of a conversion to the `Client`.
///
/// This type is pretty much a `Result`, but given that it is local to
/// our crate we can implement non-local traits for it. We exploit this
/// fact in the `Client` struct which relies on a From conversion from
/// an (HTTP status, Body)-pair yielding such a result.
#[derive(Debug)]
pub struct ConvertResult<T, E>(pub Result<T, E>);

impl<T, E> Into<Result<T, E>> for ConvertResult<T, E> {
  fn into(self) -> Result<T, E> {
    self.0
  }
}


/// A macro used for defining the properties for a request to a
/// particular HTTP endpoint.
macro_rules! EndpointDef {
  ( $name:ident,
    Ok => $out:ty, [$($ok_status:ident,)*],
    Err => $err:ident, [$($err_status:ident => $variant:ident,)*] ) => {

    EndpointDefImpl! {
      $name,
      Ok => $out, [$($ok_status,)*],
      Err => $err, [
        // Every request can result in an authentication failure or fall
        // prey to the rate limit and so we include these variants into
        // all our error definitions.
        /* 401 */ UNAUTHORIZED => AuthenticationFailed,
        /* 429 */ TOO_MANY_REQUESTS => RateLimitExceeded,
        $($err_status => $variant,)*
      ]
    }
  };
}

macro_rules! EndpointDefImpl {
  ( $name:ident,
    Ok => $out:ty, [$($ok_status:ident,)*],
    Err => $err:ident, [$($err_status:ident => $variant:ident,)*] ) => {

    #[allow(unused_qualifications)]
    impl ::std::convert::From<(::hyper::http::StatusCode, ::std::vec::Vec<u8>)>
      for crate::endpoint::ConvertResult<$out, $err> {

      #[allow(unused)]
      fn from(data: (::hyper::http::StatusCode, ::std::vec::Vec<u8>)) -> Self {
        let (status, body) = data;
        match status {
          $(
            ::hyper::http::StatusCode::$ok_status => {
              match $name::parse(&body) {
                Ok(obj) => crate::endpoint::ConvertResult(Ok(obj)),
                Err(err) => crate::endpoint::ConvertResult(Err(err)),
              }
            },
          )*
          $(
            ::hyper::http::StatusCode::$err_status => {
              crate::endpoint::ConvertResult(Err($err::$variant))
            },
          )*
          _ => crate::endpoint::ConvertResult(Err($err::UnexpectedStatus(status))),
        }
      }
    }

    /// An enum representing the various errors this endpoint may
    /// encounter.
    // TODO: Figure out how to make doc comments work for the dynamic
    //       variants.
    #[allow(missing_docs)]
    #[allow(unused_qualifications)]
    #[derive(Debug)]
    pub enum $err {
      $(
        $variant,
      )*
      /// An HTTP status not present in the endpoint's definition was
      /// encountered.
      UnexpectedStatus(::hyper::http::StatusCode),
      /// An error reported by the `hyper` crate.
      Hyper(::hyper::Error),
      /// A JSON conversion error.
      Json(::serde_json::Error),
    }

    #[allow(unused_qualifications)]
    impl ::std::convert::From<::hyper::Error> for $err {
      fn from(src: ::hyper::Error) -> Self {
        $err::Hyper(src)
      }
    }

    #[allow(unused_qualifications)]
    impl ::std::convert::From<::serde_json::Error> for $err {
      fn from(src: ::serde_json::Error) -> Self {
        $err::Json(src)
      }
    }

    #[allow(unused_qualifications)]
    impl ::std::convert::From<$err> for crate::Error {
      fn from(src: $err) -> Self {
        match src {
          $(
            $err::$variant => {
              crate::Error::HttpStatus(::hyper::http::StatusCode::$err_status)
            },
          )*
          $err::UnexpectedStatus(status) => crate::Error::HttpStatus(status),
          $err::Hyper(err) => crate::Error::Hyper(err),
          $err::Json(err) => crate::Error::Json(err),
        }
      }
    }
  };
}