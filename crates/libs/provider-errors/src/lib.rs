//! Provider 调用错误的标准化解析。

mod error;
pub use error::{
    ApiCallError, ParsedApiCallError, ParsedStreamError, parse_api_call_error, parse_stream_error,
};

#[cfg(test)]
mod error_tests;
