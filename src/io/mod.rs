use digest::Digest;
use pin_project::pin_project;
use std::io::{self, ErrorKind, Read};
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::{channel, Receiver, SendError, Sender};
use {
    std::pin::Pin,
    std::task::{Context, Poll},
    tokio::io::AsyncRead,
};

#[pin_project]
pub struct HashingReader<R, H: Digest> {
    #[pin]
    reader: R,
    hasher: H,
    chan: Sender<Option<Vec<u8>>>,
    //Is this the best way to check EOF based on file length?
    size: AtomicUsize,
}

impl<R, H> HashingReader<R, H>
where
    H: Digest,
{
    pub fn new(
        reader: R,
        size: usize,
    ) -> (Self, Sender<Option<Vec<u8>>>, Receiver<Option<Vec<u8>>>) {
        let (tx, rx) = channel::<Option<Vec<u8>>>();
        //Need to return cloned sender, users should store it, otherwise the channel will be closed
        //before they have a chance to read anything
        let tx_clone = tx.clone();
        let hr: HashingReader<R, H> = HashingReader {
            reader,
            hasher: H::new(),
            chan: tx_clone,
            size: AtomicUsize::new(size),
        };
        (hr, tx, rx)
    }
}

impl<R, H> Read for HashingReader<R, H>
where
    R: Read,
    H: Digest,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = match self.reader.read(buf) {
            Ok(len) => len,
            Err(e) => {
                self.chan.send(None).map_err(channel_error)?;
                return Err(e);
            }
        };
        if len == 0 {
            let hasher = std::mem::replace(&mut self.hasher, H::new());
            self.chan
                .send(Some(hasher.finalize().as_slice().to_vec()))
                .map_err(channel_error)?;
        } else {
            self.hasher.update(&buf[..len]);
        }
        Ok(len)
    }
}

impl<R, H> AsyncRead for HashingReader<R, H>
where
    R: AsyncRead + Send + Unpin,
    H: Digest + digest::Reset,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::result::Result<(), io::Error>> {
        let mut this = self.project();
        let filled_before = buf.filled().len();
        match this.reader.as_mut().poll_read(cx, buf) {
            Poll::Ready(Ok(())) => {
                let filled_after = buf.filled().len();
                if filled_before == filled_after {
                    let hasher = std::mem::replace(this.hasher, H::new());
                    this.chan
                        .send(Some(hasher.finalize().as_slice().to_vec()))
                        .map_err(channel_error)?;
                } else {
                    let newly_filled = &buf.filled()[filled_before..filled_after];
                    this.hasher.update(newly_filled);

                    let len: usize = newly_filled.len();
                    this.size
                        .fetch_sub(len, std::sync::atomic::Ordering::SeqCst);
                    //Not every implementation calls Poll:Ready to check for EOF
                    if this.size.load(std::sync::atomic::Ordering::SeqCst) == 0 {
                        let hasher = std::mem::replace(this.hasher, H::new());
                        this.chan
                            .send(Some(hasher.finalize().as_slice().to_vec()))
                            .map_err(channel_error)?;
                    }
                }
                Poll::Ready(Ok(()))
            }
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => {
                this.chan.send(None).map_err(channel_error)?;
                Poll::Ready(Err(e))
            }
        }
    }
}

fn channel_error<T>(e: SendError<T>) -> io::Error {
    io::Error::new(
        ErrorKind::Other,
        format!("EOF reached but was unable to send hash: {:?}", e),
    )
}
