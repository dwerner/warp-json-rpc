use crate::req::Version;
use hyper::{Body, Response as HyperResponse};
use serde::Serialize;
use serde_json::Value;
use std::borrow::Cow;

/*
 * ========
 * Response
 * ========
 */
#[derive(PartialEq, Debug, Serialize)]
pub struct Response {
    pub jsonrpc: Version,
    pub id: Option<u64>,
    #[serde(flatten)]
    pub content: ResponseContent,
}

impl Response {
    pub fn new(id: Option<u64>, success: impl Serialize) -> Response {
        Response {
            jsonrpc: Version::V2,
            id,
            content: ResponseContent::Success(serde_json::to_value(success).unwrap()),
        }
    }

    pub fn new_err(id: Option<u64>, error: Error) -> Response {
        Response {
            jsonrpc: Version::V2,
            id,
            content: ResponseContent::Error(error),
        }
    }
}

impl<'a> Into<HyperResponse<Body>> for &'a Response {
    fn into(self) -> HyperResponse<Body> {
        let body = Body::from(serde_json::to_vec(self).unwrap());
        HyperResponse::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(body)
            .unwrap()
    }
}

#[derive(PartialEq, Debug, Serialize)]
pub enum ResponseContent {
    #[serde(rename = "result")]
    Success(Value),
    #[serde(rename = "error")]
    Error(Error),
}

#[derive(PartialEq, Debug, Serialize)]
pub struct Error {
    pub code: i64,
    pub message: Cow<'static, str>,
    pub data: Option<Value>,
}

impl Error {
    pub const PARSE_ERROR: Error = Error {
        code: -32700,
        message: Cow::Borrowed("Parse error"),
        data: None,
    };

    pub const INVALID_REQUEST: Error = Error {
        code: -32600,
        message: Cow::Borrowed("Invalid Request"),
        data: None,
    };

    pub const METHOD_NOT_FOUND: Error = Error {
        code: -32601,
        message: Cow::Borrowed("Method not found"),
        data: None,
    };

    pub const INVALID_PARAMS: Error = Error {
        code: -32602,
        message: Cow::Borrowed("Invalid params"),
        data: None,
    };

    pub const INTERNAL_ERROR: Error = Error {
        code: -32603,
        message: Cow::Borrowed("Internal error"),
        data: None,
    };

    pub fn custom<S>(code: i64, message: S, data: Option<impl Serialize>) -> Error
    where
        Cow<'static, str>: From<S>,
    {
        Error {
            code,
            message: message.into(),
            data: data.map(|s| serde_json::to_value(s).unwrap()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn serialize_response() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct Expected {
            jsonrpc: String,
            result: String,
            id: usize,
        }

        let res = Response::new(Some(42), "The answer");
        let res_str = serde_json::to_string(&res).unwrap();
        let deserialized = serde_json::from_str::<Expected>(res_str.as_str()).unwrap();

        let expected = Expected {
            jsonrpc: "2.0".to_string(),
            result: "The answer".to_string(),
            id: 42,
        };

        assert_eq!(deserialized, expected);
    }

    #[test]
    fn serialize_err_response() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct Expected {
            jsonrpc: String,
            error: ExpectedError,
            id: usize,
        }
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct ExpectedError {
            code: isize,
            message: String,
        }

        let res = Response::new_err(Some(42), Error::INVALID_PARAMS);
        let res_str = serde_json::to_string(&res).unwrap();
        let deserialized = serde_json::from_str::<Expected>(res_str.as_str()).unwrap();

        let expected = Expected {
            jsonrpc: "2.0".to_string(),
            error: ExpectedError {
                code: -32602,
                message: "Invalid params".to_string(),
            },
            id: 42,
        };

        assert_eq!(deserialized, expected);
    }
}
