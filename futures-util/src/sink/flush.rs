use core::marker::PhantomData;
use core::pin::Pin;
use futures_core::future::Future;
use futures_core::task::{Waker, Poll};
use futures_sink::Sink;

/// Future for the `flush` combinator, which polls the sink until all data
/// has been flushed.
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub struct Flush<'a, Si: Sink<Item> + Unpin + ?Sized, Item> {
    sink: &'a mut Si,
    _phantom: PhantomData<fn(Item)>,
}

// Pin is never projected to a field.
impl<Si: Sink<Item> + Unpin + ?Sized, Item> Unpin for Flush<'_, Si, Item> {}

/// A future that completes when the sink has finished processing all
/// pending requests.
///
/// The sink itself is returned after flushing is complete; this adapter is
/// intended to be used when you want to stop sending to the sink until
/// all current requests are processed.
impl<'a, Si: Sink<Item> + Unpin + ?Sized, Item> Flush<'a, Si, Item> {
    pub(super) fn new(sink: &'a mut Si) -> Self {
        Flush {
            sink,
            _phantom: PhantomData,
        }
    }
}

impl<Si: Sink<Item> + Unpin + ?Sized, Item> Future for Flush<'_, Si, Item> {
    type Output = Result<(), Si::SinkError>;

    fn poll(
        mut self: Pin<&mut Self>,
        waker: &Waker,
    ) -> Poll<Self::Output> {
        Pin::new(&mut self.sink).poll_flush(waker)
    }
}
