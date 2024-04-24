use std::fmt::Display;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MarketPrefix {
  Stocks,
  Crypto
}
impl Display for MarketPrefix {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      MarketPrefix::Stocks => write!(f, "/v2/stocks/"),
      MarketPrefix::Crypto => write!(f, "/v1beta3/crypto/us/")
    }
  }
}
impl Default for MarketPrefix {
  fn default() -> Self {
    MarketPrefix::Stocks
  }
}