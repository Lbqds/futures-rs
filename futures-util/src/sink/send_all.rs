use crate::stream::{StreamExt, Fuse};
use core::pin::Pin;
use futures_core::future::Future;
use futures_core::stream::Stream;
use futures_core::task::{Waker, Poll};
use futures_sink::Sink;

/// Future for the `Sink::send_all` combinator, which sends a stream of values
/// to a sink and then waits until the sink has fully flushed those values.
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub struct SendAll<'a, Si, St>
where
    Si: Sink<St::Item> + Unpin + ?Sized,
    St: Stream + Unpin + ?Sized,
{
    sink: &'a mut Si,
    stream: Fuse<&'a mut St>,
    buffered: Option<St::Item>,
}

// Pinning is never projected to any fields
impl<Si, St> Unpin for SendAll<'_, Si, St>
where
    Si: Sink<St::Item> + Unpin + ?Sized,
    St: Stream + Unpin + ?Sized,
{}

impl<'a, Si, St> SendAll<'a, Si, St>
where
    Si: Sink<St::Item> + Unpin + ?Sized,
    St: Stream + Unpin + ?Sized,
{
    pub(super) fn new(
        sink: &'a mut Si,
        stream: &'a mut St,
    ) -> SendAll<'a, Si, St> {
        SendAll {
            sink,
            stream: stream.fuse(),
            buffered: None,
        }
    }

    fn try_start_send(
        &mut self,
        waker: &Waker,
        item: St::Item,
    ) -> Poll<Result<(), Si::SinkError>> {
        debug_assert!(self.buffered.is_none());
        match Pin::new(&mut self.sink).poll_ready(waker) {
            Poll::Ready(Ok(())) => {
                Poll::Ready(Pin::new(&mut self.sink).start_send(item))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => {
                self.buffered = Some(item);
                Poll::Pending
            }
        }
    }
}

impl<Si, St> Future for SendAll<'_, Si, St>
where
    Si: Sink<St::Item> + Unpin + ?Sized,
    St: Stream + Unpin + ?Sized,
{
    type Output = Result<(), Si::SinkError>;

    fn poll(
        mut self: Pin<&mut Self>,
        waker: &Waker,
    ) -> Poll<Self::Output> {
        let this = &mut *self;
        // If we've got an item buffered already, we need to write it to the
        // sink before we can do anything else
        if let Some(item) = this.buffered.take() {
            try_ready!(this.try_start_send(waker, item))
        }

        loop {
            match this.stream.poll_next_unpin(waker) {
                Poll::Ready(Some(item)) => {
                    try_ready!(this.try_start_send(waker, item))
                }
                Poll::Ready(None) => {
                    try_ready!(Pin::new(&mut this.sink).poll_flush(waker));
                    return Poll::Ready(Ok(()))
                }
                Poll::Pending => {
                    try_ready!(Pin::new(&mut this.sink).poll_flush(waker));
                    return Poll::Pending
                }
            }
        }
    }
}
