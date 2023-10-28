use std::{
    cmp,
    io::{self, Read},
    sync::{Arc, Condvar, LockResult, Mutex, MutexGuard},
    task::Poll,
    thread::JoinHandle,
};
use tokio::io::{AsyncRead, ReadBuf};

use crate::error;

macro_rules! uninterruptibly {
    ($e:expr) => {{
        loop {
            match $e {
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                res => break res,
            }
        }
    }};
}

const MAX_BUF: usize = 1024 * 1024;

#[derive(Debug, Clone)]
struct Buf {
    buf: Vec<u8>,
    pos: usize,
}

impl Buf {
    fn with_capacity(capacity: usize) -> Self {
        Self { buf: Vec::with_capacity(capacity), pos: 0 }
    }

    fn copy_to(&mut self, dst: &mut ReadBuf<'_>) -> usize {
        let n = cmp::min(self.len(), dst.remaining());
        dst.put_slice(&self.bytes()[..n]);
        self.pos += n;

        if self.pos == self.buf.len() {
            self.buf.truncate(0);
            self.pos = 0;
        }

        n
    }

    fn read_from<T: Read>(&mut self, rd: &mut T) -> io::Result<usize> {
        let res = uninterruptibly!(rd.read(&mut self.buf));

        if let Ok(n) = res {
            self.buf.truncate(n);
        } else {
            self.buf.clear();
        }

        assert_eq!(self.pos, 0);

        res
    }

    fn ensure_capacity_for(&mut self, bytes: &ReadBuf<'_>) {
        assert!(self.is_empty());

        let len = cmp::min(bytes.remaining(), MAX_BUF);

        if self.buf.len() < len {
            self.buf.reserve(len - self.buf.len());
        }

        unsafe {
            self.buf.set_len(len);
        }
    }

    fn bytes(&self) -> &[u8] {
        &self.buf[self.pos..]
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn len(&self) -> usize {
        self.buf.len() - self.pos
    }
}

struct Locked<T>(Arc<(Mutex<T>, Condvar)>);

impl<T> Locked<T> {
    fn new(val: T) -> Self {
        Self(Arc::new((Mutex::new(val), Condvar::new())))
    }

    fn lock(&self) -> LockResult<MutexGuard<'_, T>> {
        let (lock, _) = self.0.as_ref();
        lock.lock()
    }

    fn wait<'a>(&self, guard: MutexGuard<'a, T>) -> LockResult<MutexGuard<'a, T>> {
        let (_, cvar) = self.0.as_ref();
        cvar.wait(guard)
    }

    fn update<'a>(&self, mut guard: MutexGuard<'a, T>, val: T) -> MutexGuard<'a, T> {
        *guard = val;
        guard
    }

    fn update_and_notify<'a>(&self, guard: MutexGuard<'a, T>, val: T) -> MutexGuard<'a, T> {
        let guard = self.update(guard, val);
        let (_, cvar) = self.0.as_ref();
        cvar.notify_one();
        guard
    }
}

impl<T> Clone for Locked<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Debug)]
enum State {
    Idle(Option<Buf>),
    Busy(Option<Buf>, std::task::Waker),
    Ready(Option<Buf>, Option<io::Result<usize>>),
    Dropped,
}

pub struct AsyncStdin {
    state_lock: Locked<State>,
    thread_handle: Option<JoinHandle<error::Result<()>>>,
}

impl AsyncStdin {
    pub fn new() -> Self {
        let buf = Buf::with_capacity(0);
        let state_lock = Locked::new(State::Idle(Some(buf)));
        let thread_handle = Some(Self::spawn_stdin_read(state_lock.clone()));
        Self { state_lock, thread_handle }
    }

    fn spawn_stdin_read(state_lock: Locked<State>) -> JoinHandle<Result<(), error::Error>> {
        std::thread::spawn(move || {
            let mut stdin = io::stdin();
            'main: loop {
                // wait until state is busy.
                let mut buf: Option<Buf>;
                let mut waker: Option<std::task::Waker>;
                {
                    let mut state = state_lock.lock()?;
                    'wait_state: loop {
                        match *state {
                            State::Dropped => break 'main,
                            State::Busy(ref mut b, ref w) => {
                                buf = Some(b.take().unwrap());
                                waker = Some(w.clone());
                                break 'wait_state;
                            }
                            _ => state = state_lock.wait(state)?,
                        }
                    }
                }

                let mut buf = buf.take().unwrap();
                let read = buf.read_from(&mut stdin);
                let state = state_lock.lock()?;
                drop(state_lock.update(state, State::Ready(Some(buf), Some(read))));
                waker.take().unwrap().wake_by_ref();
            }
            Ok::<(), crate::error::Error>(())
        })
    }
}

impl Drop for AsyncStdin {
    fn drop(&mut self) {
        match self.state_lock.lock() {
            Ok(mut state) => {
                state = self.state_lock.update_and_notify(state, State::Dropped);
                drop(state); // release the lock

                match self.thread_handle.take() {
                    Some(thread_handle) => _ = thread_handle.join(),
                    None => {}
                }
            }
            Err(_) => {}
        }
    }
}

impl AsyncRead for AsyncStdin {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        dst: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.state_lock.lock() {
            Ok(mut guard) => loop {
                match *guard {
                    State::Idle(ref mut buf_cell) => {
                        assert!(!self.thread_handle.as_ref().unwrap().is_finished());
                        let mut buf = buf_cell.take().unwrap();
                        if !buf.is_empty() {
                            buf.copy_to(dst);
                            *buf_cell = Some(buf);
                            return Poll::Ready(Ok(()));
                        }

                        buf.ensure_capacity_for(dst);
                        guard = self
                            .state_lock
                            .update_and_notify(guard, State::Busy(Some(buf), cx.waker().clone()))
                    }
                    State::Busy(_, _) => {
                        assert!(!self.thread_handle.as_ref().unwrap().is_finished());
                        return Poll::Pending;
                    }
                    State::Ready(ref mut buf_cell, ref mut res_cell) => {
                        let res = res_cell.take().unwrap();
                        let mut buf = buf_cell.take().unwrap();
                        match res {
                            Ok(_) => {
                                buf.copy_to(dst);
                                let guard = self
                                    .state_lock
                                    .update_and_notify(guard, State::Idle(Some(buf)));
                                drop(guard);
                                return Poll::Ready(Ok(()));
                            }
                            Err(e) => {
                                assert!(buf.is_empty());
                                let guard = self
                                    .state_lock
                                    .update_and_notify(guard, State::Idle(Some(buf)));
                                drop(guard);
                                return Poll::Ready(Err(e));
                            }
                        }
                    }
                    State::Dropped => unreachable!(),
                };
            },
            Err(e) => Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e.to_string()))),
        }
    }
}
