use bytes::Bytes;
use reqwest::{multipart::Form, Client, Response};
use std::{
    future::Future,
    io,
    io::{Cursor, Read},
    ops::DerefMut,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::prelude::AsyncRead;

// Relevant endpoints and HTTP fields:
//
// ## Endpoints
// http://localhost:8080/crumbIssuer/api/json
// http://localhost:8080/job/foo/2/logText/progressiveText --data "start=<offset>"
//
// ## Headers
// X-Text-Size: 429280
// X-More-Data: true
//
// ## reqwest stuff
// (send request right away) https://docs.rs/reqwest/0.10.7/reqwest/struct.RequestBuilder.html#method.send
// (multipart form body) https://docs.rs/reqwest/0.10.7/reqwest/multipart/struct.Form.html

/// Request response future.
type ResponseFuture = Box<dyn Future<Output = reqwest::Result<Response>> + Unpin>;
/// Request body future.
type BytesFuture = Box<dyn Future<Output = reqwest::Result<Bytes>> + Unpin>;

enum State {
    /// Initial state, prior to initializing a new HTTP request with the given
    /// offset. If the offset is None, the tracing has ended.
    Init(usize),
    /// Poll the response of the HTTP request.
    ResponseFuture(ResponseFuture),
    /// Poll the body of the HTTP response.
    BodyFuture {
        /// "X-Text-Size" HTTP header:
        text_size: usize,
        /// "X-More-Data" HTTP header.
        more_data: bool,
        body_future: BytesFuture,
    },
    /// Return body and so on
    Body {
        /// Next Init offset.
        offset: Option<usize>,
        body: Cursor<Bytes>,
    },
}

/// A type to read the log of a given jenkins build.
/// This type implements [`AsyncRead`] so it can be used with the tokio::io
/// toolset.
///
/// [`AsyncRead`]: https://docs.rs/tokio/*/tokio/io/trait.AsyncRead.html
pub struct JenkinsTrace {
    client: Client,
    state: Option<State>,
    // To keep track of the # of bytes read so far.
    offset: usize,
    ended: bool,
}

impl JenkinsTrace {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            state: Some(State::Init(0)),
            offset: 0,
            ended: false,
        }
    }

    // handle init state
    fn init(mut self: Pin<&mut Self>, offset: usize) {
        let form = Form::new().text("start", format!("{}", offset));
        let response = self
            .client
            .get("http://localhost:8080/job/foo/2/logText/progressiveText")
            .multipart(form)
            .header("Authorization", "Basic cm9vdDpyb290")
            .header(
                "Jenkins-Crumb",
                "2e90e384fa7a0e3aca1a56d02da8f9bd1a65f933876ebc8886bf43aa72bff7e7",
            )
            .send();

        // update state
        self.state = Some(State::ResponseFuture(Box::new(response)));
    }

    // Drive Response future to completion
    fn response_future(
        &mut self,
        mut response: ResponseFuture,
        cx: &mut Context,
    ) -> Poll<Option<reqwest::Error>> {
        match Pin::new(response.deref_mut()).poll(cx) {
            Poll::Pending => {
                // update state
                self.state = Some(State::ResponseFuture(response));
                Poll::Pending
            }
            Poll::Ready(Err(err)) => Poll::Ready(Some(err)),
            Poll::Ready(Ok(response)) => {
                self.state = Some(State::BodyFuture {
                    body_future: Box::new(response.bytes()),
                    text_size: 0,
                    more_data: false,
                });
                Poll::Ready(None)
            }
        }
    }

    // drive Body future to completion
    fn body_future(
        &mut self,
        text_size: usize,
        more_data: bool,
        body_future: BytesFuture,
        cx: &mut Context,
    ) {
        unimplemented!()
    }
}

// impl AsyncRead for JenkinsTrace {
//     fn poll_read(
//         self: Pin<&mut Self>,
//         cx: &mut Context,
//         buf: &mut [u8],
//     ) -> Poll<io::Result<usize>> {
//         if self.ended {
//             return Poll::Ready(Ok(0));
//         }
//
//         loop {
//             match self.state.take() {
//                 // EOF
//                 None => return Poll::Ready(Ok(0)),
//                 Some(State::Init(offset)) => self.init(offset),
//                 Some(State::ResponseFuture(mut response)) => {
//                     match self.response_future(response, cx) {
//                         Poll::Pending => return Poll::Pending,
//                         Poll::Ready(Some(err)) => {
//                             self.ended = true;
//
//                             return Poll::Ready(Err(err));
//                         }
//                         Poll::Ready(None) => { /* move on to BodyFuture */ }
//                     }
//                 }
//                 Some(State::BodyFuture {
//                     text_size,
//                     more_data,
//                     body_future,
//                 }) => {
//                     let body_future = body_future;
//                     self.body_future(text_size, more_data, body_future, cx);
//                 }
//                 Some(State::Body { offset, body }) => {}
//             }
//         }
//     }
// }

#[tokio::main]
async fn main() {}
