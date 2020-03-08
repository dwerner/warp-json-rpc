use crate::req::Version;
use hyper::Body;
use serde::Serialize;
use std::borrow::Cow;

/*
 * ========
 * Response
 * ========
 */
#[derive(Serialize)]
struct Response {
    jsonrpc: Version,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u64>,
    #[serde(flatten)]
    content: ResponseContent,
}

impl Response {
    fn new(id: Option<u64>, content: ResponseContent) -> Response {
        Response {
            jsonrpc: Version::V2,
            id,
            content,
        }
    }

    /// Currently `warp` does not expose `Reply` trait (it is guarded).
    /// So we need to convert this into something that implements `Reply`.
    fn into_reply(self) -> anyhow::Result<http::Response<Body>> {
        let body = Body::from(serde_json::to_vec(&self)?);
        Ok(http::Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(body)
            .unwrap())
    }
}

pub struct Builder {
    id: Option<u64>,
}

impl Builder {
    pub(crate) fn new(id: Option<u64>) -> Builder {
        Builder { id }
    }

    pub fn success<S>(self, content: S) -> anyhow::Result<http::Response<Body>>
    where
        S: Serialize + 'static,
    {
        Response::new(self.id, ResponseContent::Success(Box::new(content))).into_reply()
    }

    pub fn error(self, error: Error) -> anyhow::Result<http::Response<Body>> {
        Response::new(self.id, ResponseContent::Error(error)).into_reply()
    }

    pub fn result<S>(self, result: Result<S, Error>) -> anyhow::Result<http::Response<Body>>
    where
        S: Serialize + 'static,
    {
        match result {
            Ok(success) => self.success(success),
            Err(error) => self.error(error),
        }
    }
}

#[derive(Serialize)]
enum ResponseContent {
    #[serde(rename = "result")]
    Success(Box<dyn erased_serde::Serialize>),
    #[serde(rename = "error")]
    Error(Error),
}

#[derive(Serialize)]
pub struct Error {
    pub code: i64,
    pub message: Cow<'static, str>,
    pub data: Option<Box<dyn erased_serde::Serialize>>,
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

    pub fn custom<S>(code: i64, message: S, data: Option<impl Serialize + 'static>) -> Error
    where
        Cow<'static, str>: From<S>,
    {
        Error {
            code,
            message: message.into(),
            data: data.map(|s| Box::new(s) as Box<dyn erased_serde::Serialize>),
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

        let res = Response::new(Some(42), ResponseContent::Success(Box::new("The answer")));
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

        let res = Response::new(Some(42), ResponseContent::Error(Error::INVALID_PARAMS));
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

    #[test]
    fn serialize_no_id_response_shoud_not_contain_id_field() {
        let res = Response::new(None, ResponseContent::Success(Box::new(42)));
        let res_str = serde_json::to_string(&res).unwrap();

        assert!(!res_str.contains("id"));
    }
}
