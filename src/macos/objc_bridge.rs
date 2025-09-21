use crate::{Result, TilleRSError};
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};

/// Messages dispatched to the background Objective-C run loop thread
enum RunLoopMessage {
    Execute(Box<dyn FnOnce() + Send + 'static>),
    Shutdown,
}

/// Handle to a background thread that mimics the macOS run loop for objc bridges
pub struct RunLoopHandle {
    name: String,
    sender: Sender<RunLoopMessage>,
    join_handle: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for RunLoopHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunLoopHandle")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl RunLoopHandle {
    /// Dispatch work to the run loop thread
    pub fn dispatch<F>(&self, work: F) -> Result<()>
    where
        F: FnOnce() + Send + 'static,
    {
        self.sender
            .send(RunLoopMessage::Execute(Box::new(work)))
            .map_err(|_| {
                TilleRSError::MacOSAPIError(format!(
                    "Failed to dispatch work onto run loop '{}'",
                    self.name
                ))
            })?;

        Ok(())
    }

    /// Signal the run loop to terminate and wait for the background thread
    pub fn shutdown(mut self) -> Result<()> {
        self.sender.send(RunLoopMessage::Shutdown).map_err(|_| {
            TilleRSError::MacOSAPIError(format!(
                "Failed to signal shutdown for run loop '{}'",
                self.name
            ))
        })?;

        if let Some(handle) = self.join_handle.take() {
            handle.join().map_err(|_| {
                TilleRSError::MacOSAPIError(format!(
                    "Run loop '{}' panicked during shutdown",
                    self.name
                ))
            })?;
        }

        Ok(())
    }
}

/// Spawn a background run loop thread and return a handle for dispatching work
pub fn spawn_run_loop(name: impl Into<String>) -> Result<RunLoopHandle> {
    let name = name.into();
    let (sender, receiver) = mpsc::channel::<RunLoopMessage>();
    let thread_name = name.clone();

    let join_handle = thread::Builder::new()
        .name(thread_name.clone())
        .spawn(move || {
            while let Ok(message) = receiver.recv() {
                match message {
                    RunLoopMessage::Execute(work) => work(),
                    RunLoopMessage::Shutdown => break,
                }
            }
        })
        .map_err(|err| {
            TilleRSError::MacOSAPIError(format!(
                "Failed to spawn run loop '{}' thread: {}",
                thread_name, err
            ))
        })?;

    Ok(RunLoopHandle {
        name,
        sender,
        join_handle: Some(join_handle),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn run_loop_executes_dispatched_work() {
        let handle = spawn_run_loop("test").unwrap();
        let counter = Arc::new(Mutex::new(0));
        let cloned = counter.clone();

        handle
            .dispatch(move || {
                let mut guard = cloned.lock().unwrap();
                *guard += 1;
            })
            .unwrap();

        // Allow background thread to run
        std::thread::sleep(std::time::Duration::from_millis(10));

        assert_eq!(*counter.lock().unwrap(), 1);

        handle.shutdown().unwrap();
    }

    #[test]
    fn shutdown_waits_for_thread() {
        let handle = spawn_run_loop("test_shutdown").unwrap();
        handle.shutdown().unwrap();
    }
}
