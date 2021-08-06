use super::bufreader::DEFAULT_BUF_SIZE;
use futures::io::{AsyncBufRead, AsyncRead, AsyncSeek, AsyncWrite, IoSliceMut, SeekFrom};
use futures::ready;
use futures::task::{Context, Poll};
use pin_project_lite::pin_project;
use std::io::{self, Read};
use std::pin::Pin;
use std::{cmp, fmt};

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
    /// **Note: this is a fork from `std::io::BufReader` that reads more data in
    /// `fill_buf` even if there is already some data in the buffer**
    ///
    /// [`AsyncRead`]: futures_io::AsyncRead
    ///
    // TODO: Examples
    pub struct BufReader<R> {
        #[pin]
        inner: R,
        buffer: Vec<u8>,
        pos: usize,
        cap: usize,
    }
}

impl<R: AsyncRead> BufReader<R> {
    /// Creates a new `BufReader` with a default buffer capacity. The default is currently 8 KB,
    /// but may change in the future.
    pub fn new(inner: R) -> Self {
        Self::with_capacity(DEFAULT_BUF_SIZE, inner)
    }

    /// Creates a new `BufReader` with the specified buffer capacity.
    pub fn with_capacity(capacity: usize, inner: R) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        buffer.extend(std::iter::repeat(0).take(capacity));
        Self {
            inner,
            buffer,
            pos: 0,
            cap: 0,
        }
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
    pub fn buffer(&self) -> &[u8] {
        &self.buffer[self.pos..self.cap]
    }

    /// Invalidates all data in the internal buffer.
    #[inline]
    fn discard_buffer(self: Pin<&mut Self>) {
        let this = self.project();
        *this.pos = 0;
        *this.cap = 0;
    }
}

impl<R: AsyncRead> AsyncRead for BufReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        // If we don't have any buffered data and we're doing a massive read
        // (larger than our internal buffer), bypass our internal buffer
        // entirely.
        if self.pos == self.cap && buf.len() >= self.buffer.len() {
            let res = ready!(self.as_mut().project().inner.poll_read(cx, buf));
            self.discard_buffer();
            return Poll::Ready(res);
        }
        let mut rem = ready!(self.as_mut().poll_fill_buf(cx))?;
        let nread = rem.read(buf)?;
        self.consume(nread);
        Poll::Ready(Ok(nread))
    }

    fn poll_read_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<io::Result<usize>> {
        let total_len = bufs.iter().map(|b| b.len()).sum::<usize>();
        if self.pos == self.cap && total_len >= self.buffer.len() {
            let res = ready!(self.as_mut().project().inner.poll_read_vectored(cx, bufs));
            self.discard_buffer();
            return Poll::Ready(res);
        }
        let mut rem = ready!(self.as_mut().poll_fill_buf(cx))?;
        let nread = rem.read_vectored(bufs)?;
        self.consume(nread);
        Poll::Ready(Ok(nread))
    }

    // we can't skip unconditionally because of the large buffer case in read.
    #[cfg(feature = "read-initializer")]
    unsafe fn initializer(&self) -> Initializer {
        self.inner.initializer()
    }
}

impl<R: AsyncRead> AsyncBufRead for BufReader<R> {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<&[u8]>> {
        let this = self.project();

        if *this.cap == this.buffer.len() {
            if *this.pos == 0 {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "buffer completely filled",
                )));
            } else {
                // reset buffer position
                if *this.cap - *this.pos > 0 {
                    for i in 0..(*this.cap - *this.pos) {
                        this.buffer[i] = this.buffer[*this.pos + i];
                    }
                }
                *this.cap = *this.cap - *this.pos;
                *this.pos = 0;
            }
        }

        let read = ready!(this.inner.poll_read(cx, this.buffer))?;
        *this.cap += read;

        Poll::Ready(Ok(&this.buffer[*this.pos..*this.cap]))
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        *self.project().pos = cmp::min(self.pos + amt, self.cap);
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
            .field(
                "buffer",
                &format_args!("{}/{}", self.cap - self.pos, self.buffer.len()),
            )
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
            let remainder = (self.cap - self.pos) as i64;
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
                self.as_mut().discard_buffer();
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
        self.discard_buffer();
        Poll::Ready(Ok(result))
    }
}

