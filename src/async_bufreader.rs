use super::bufreader::DEFAULT_BUF_SIZE;
use futures_io::{AsyncBufRead, AsyncRead, AsyncSeek, AsyncWrite, IoSliceMut, SeekFrom};
use futures_util::ready;
use futures_util::AsyncReadExt;
use pin_project_lite::pin_project;
use std::collections::VecDeque;
use std::fmt;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

pin_project! {
    /// The `BufReader` struct adds buffering to any reader.
    ///
    /// It can be excessively inefficient to work directly with a [`AsyncRead`]
    /// instance. A `BufReader` performs large, infrequent reads on the underlying
    /// [`AsyncRead`] and maintains an in-memory buffer of the results.
    ///
    /// `BufReader` can improve the speed of programs that make *small* and
    /// *repeated* read calls to the same file or network socket. It does not
    /// help when reading very large amounts at once, or reading just one or a few
    /// times. It also provides no advantage when reading from a source that is
    /// already in memory, like a `Vec<u8>`.
    ///
    /// When the `BufReader` is dropped, the contents of its buffer will be
    /// discarded. Creating multiple instances of a `BufReader` on the same
    /// stream can cause data loss.
    ///
    /// [`AsyncRead`]: futures_io::AsyncRead
    ///
    // TODO: Examples
    pub struct BufReader<R> {
        #[pin]
        inner: R,
        buffer: VecDeque<u8>,
    }
}

impl<R: AsyncBufRead + std::marker::Unpin> BufReader<R> {
    /// Creates a new `BufReader` with a default buffer capacity. The default is currently 8 KB,
    /// but may change in the future.
    pub fn new(inner: R) -> Self {
        Self::with_capacity(DEFAULT_BUF_SIZE, inner)
    }

    /// Creates a new `BufReader` with the specified buffer capacity.
    pub fn with_capacity(capacity: usize, inner: R) -> Self {
        let buffer = VecDeque::with_capacity(capacity);
        Self { inner, buffer }
    }

    /// Acquires a reference to the underlying sink or stream that this combinator is
    /// pulling from.
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Acquires a mutable reference to the underlying sink or stream that this
    /// combinator is pulling from.
    ///
    /// Note that care must be taken to avoid tampering with the state of the
    /// sink or stream which may otherwise confuse this combinator.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Acquires a pinned mutable reference to the underlying sink or stream that this
    /// combinator is pulling from.
    ///
    /// Note that care must be taken to avoid tampering with the state of the
    /// sink or stream which may otherwise confuse this combinator.
    pub fn get_pin_mut(self: core::pin::Pin<&mut Self>) -> core::pin::Pin<&mut R> {
        self.project().inner
    }

    /// Consumes this combinator, returning the underlying sink or stream.
    ///
    /// Note that this may discard intermediate state of this combinator, so
    /// care should be taken to avoid losing resources when this is called.
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Returns a reference to the internally buffered data.
    ///
    /// Unlike `fill_buf`, this will not attempt to fill the buffer if it is empty.
    pub fn buffer(&mut self) -> &[u8] {
        self.buffer.make_contiguous()
    }

    pub fn consume(&mut self, nread: usize) {
        for _ in 0..nread {
            self.buffer.pop_front();
        }
    }

    pub async fn my_fill_buf(
        &mut self,
        nread: Option<std::num::NonZeroUsize>,
    ) -> Result<usize, io::Error> {
        let mut buf = match nread {
            Some(x) => vec![0; x.get().next_power_of_two()],
            None => vec![0; 4096],
        };

        let i = self.inner.read(&mut buf).await?;
        buf.truncate(i);

        self.buffer.append(&mut buf.into());

        Ok(i)
    }
}

impl<R: AsyncBufRead + std::marker::Unpin> AsyncRead for BufReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.project();
        // If we don't have any buffered data and we're doing a massive read
        // (larger than our internal buffer), bypass our internal buffer
        // entirely.
        if buf.len() >= this.buffer.len() {
            let res = ready!(this.inner.poll_read(cx, buf));
            this.buffer.clear();
            return Poll::Ready(res);
        }
        let mut rem = ready!(this.inner.poll_fill_buf(cx))?;
        let nread = std::io::Read::read(&mut rem, buf)?;
        for _ in 0..nread {
            this.buffer.pop_front();
        }
        Poll::Ready(Ok(nread))
    }

    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<io::Result<usize>> {
        let this = self.project();
        let total_len = bufs.iter().map(|b| b.len()).sum::<usize>();
        if total_len >= this.buffer.len() {
            let res = ready!(this.inner.poll_read_vectored(cx, bufs));
            this.buffer.clear();
            return Poll::Ready(res);
        }
        let mut rem = ready!(this.inner.poll_fill_buf(cx))?;
        let nread = std::io::Read::read_vectored(&mut rem, bufs)?;
        for _ in 0..nread {
            this.buffer.pop_front();
        }
        Poll::Ready(Ok(nread))
    }

    // we can't skip unconditionally because of the large buffer case in read.
    #[cfg(feature = "read-initializer")]
    unsafe fn initializer(&self) -> Initializer {
        self.inner.initializer()
    }
}

impl<R: AsyncWrite> AsyncWrite for BufReader<R> {
    fn poll_write(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
        buf: &[u8],
    ) -> core::task::Poll<std::io::Result<usize>> {
        self.project().inner.poll_write(cx, buf)
    }
    fn poll_write_vectored(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> core::task::Poll<std::io::Result<usize>> {
        self.project().inner.poll_write_vectored(cx, bufs)
    }
    fn poll_flush(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<std::io::Result<()>> {
        self.project().inner.poll_flush(cx)
    }
    fn poll_close(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<std::io::Result<()>> {
        self.project().inner.poll_close(cx)
    }
}

impl<R: fmt::Debug> fmt::Debug for BufReader<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BufReader")
            .field("reader", &self.inner)
            .field("buffer", &format_args!("{}", self.buffer.len()))
            .finish()
    }
}

impl<R: AsyncRead + AsyncSeek> AsyncSeek for BufReader<R> {
    /// Seek to an offset, in bytes, in the underlying reader.
    ///
    /// The position used for seeking with `SeekFrom::Current(_)` is the
    /// position the underlying reader would be at if the `BufReader` had no
    /// internal buffer.
    ///
    /// Seeking always discards the internal buffer, even if the seek position
    /// would otherwise fall within it. This guarantees that calling
    /// `.into_inner()` immediately after a seek yields the underlying reader
    /// at the same position.
    ///
    /// See [`AsyncSeek`](futures_io::AsyncSeek) for more details.
    ///
    /// Note: In the edge case where you're seeking with `SeekFrom::Current(n)`
    /// where `n` minus the internal buffer length overflows an `i64`, two
    /// seeks will be performed instead of one. If the second seek returns
    /// `Err`, the underlying reader will be left at the same position it would
    /// have if you called `seek` with `SeekFrom::Current(0)`.
    fn poll_seek(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<io::Result<u64>> {
        let result: u64;
        if let SeekFrom::Current(n) = pos {
            let remainder = self.as_mut().project().buffer.len() as i64;
            // it should be safe to assume that remainder fits within an i64 as the alternative
            // means we managed to allocate 8 exbibytes and that's absurd.
            // But it's not out of the realm of possibility for some weird underlying reader to
            // support seeking by i64::min_value() so we need to handle underflow when subtracting
            // remainder.
            if let Some(offset) = n.checked_sub(remainder) {
                result = ready!(self
                    .as_mut()
                    .project()
                    .inner
                    .poll_seek(cx, SeekFrom::Current(offset)))?;
            } else {
                // seek backwards by our remainder, and then by the offset
                ready!(self
                    .as_mut()
                    .project()
                    .inner
                    .poll_seek(cx, SeekFrom::Current(-remainder)))?;
                self.as_mut().project().buffer.clear();
                result = ready!(self
                    .as_mut()
                    .project()
                    .inner
                    .poll_seek(cx, SeekFrom::Current(n)))?;
            }
        } else {
            // Seeking with Start/End doesn't care about our buffer length.
            result = ready!(self.as_mut().project().inner.poll_seek(cx, pos))?;
        }
        self.as_mut().project().buffer.clear();
        Poll::Ready(Ok(result))
    }
}
