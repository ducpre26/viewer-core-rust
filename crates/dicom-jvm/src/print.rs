//! Each print transaction owns its native thread and JVM attachment.
use super::*;
use std::{
    sync::{Condvar, Mutex},
    time::Instant,
};

struct Calls {
    vm: Option<Arc<JavaVM>>,
    active: usize,
}
pub(super) struct Runtime {
    calls: Mutex<Calls>,
    completed: Condvar,
    health: watch::Sender<State>,
}
struct Active(Arc<Runtime>);
impl Drop for Active {
    fn drop(&mut self) {
        let mut calls = self
            .0
            .calls
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        calls.active -= 1;
        self.0.completed.notify_all();
    }
}
impl Runtime {
    pub fn new(vm: Arc<JavaVM>, health: watch::Sender<State>) -> Arc<Self> {
        Arc::new(Self {
            calls: Mutex::new(Calls {
                vm: Some(vm),
                active: 0,
            }),
            completed: Condvar::new(),
            health,
        })
    }
    fn start(self: &Arc<Self>, job: Job) -> Result<(), Error> {
        let vm = {
            let mut calls = self.calls.lock().map_err(|_| Error::WorkerStopped)?;
            let vm = calls.vm.clone().ok_or(Error::WorkerStopped)?;
            calls.active += 1;
            vm
        };
        let active = Active(self.clone());
        let health = self.health.clone();
        std::thread::Builder::new()
            .name("dicom-print-jni".into())
            .spawn(move || {
                // The VM clone and attachment drop before Active signals shutdown.
                let _active = active;
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute(vm, job)));
                if !matches!(result, Ok(Ok(()))) {
                    health.send_replace(State::Failed);
                }
            })
            .map_err(|_| Error::Startup)?;
        Ok(())
    }
    pub fn shutdown(&self) -> Result<(), Error> {
        let mut calls = self.calls.lock().map_err(|_| Error::Shutdown)?;
        // Stop admission before waiting, including callers holding cloned handles.
        calls.vm.take();
        while calls.active > 0 {
            calls = self.completed.wait(calls).map_err(|_| Error::Shutdown)?;
        }
        Ok(())
    }
}
struct Job {
    manifest: String,
    _owner: Arc<dyn Send + Sync>,
    deadline: Instant,
    reply: oneshot::Sender<Result<String, Error>>,
}
impl Jvm {
    pub fn print_timeout(&self) -> Duration {
        self.0.timeout
    }
    pub async fn print(
        &self,
        manifest: String,
        owner: Arc<dyn Send + Sync>,
    ) -> Result<String, Error> {
        if *self.0.state.borrow() != State::Ready {
            return Err(Error::WorkerStopped);
        }
        if !self
            .0
            .capabilities
            .borrow()
            .as_ref()
            .is_some_and(|c| c.operations.iter().any(|op| op == "print"))
        {
            return Err(Error::Contract);
        }
        let runtime = self
            .0
            .print_runtime
            .borrow()
            .clone()
            .ok_or(Error::WorkerStopped)?;
        let deadline = Instant::now() + self.0.timeout;
        let (reply, receiver) = oneshot::channel();
        runtime.start(Job {
            manifest,
            _owner: owner,
            deadline,
            reply,
        })?;
        // Dropping the HTTP waiter never drops the native call's owned inputs.
        tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), receiver)
            .await
            .map_err(|_| Error::Timeout)?
            .map_err(|_| Error::WorkerStopped)?
    }
}
fn execute(vm: Arc<JavaVM>, job: Job) -> Result<(), Error> {
    if job.reply.is_closed() || Instant::now() >= job.deadline {
        let _ = job.reply.send(Err(Error::Timeout));
        return Ok(());
    }
    let mut env = vm.attach_current_thread().map_err(|_| Error::Jni)?;
    let result = frame(&mut env, |env| {
        let class = invoke(env, |env| env.find_class(FACADE))?;
        let manifest = invoke(env, |env| env.new_string(&job.manifest))?;
        let value = invoke(env, |env| {
            env.call_static_method(
                class,
                "print",
                "(Ljava/lang/String;)Ljava/lang/String;",
                &[(&manifest).into()],
            )
        })?
        .l()?;
        let text = JString::from(value);
        Ok(invoke(env, |env| env.get_string(&text))?.into())
    });
    let fatal = matches!(result, Err(Error::Unrecoverable));
    let _ = job.reply.send(result);
    if fatal {
        Err(Error::Unrecoverable)
    } else {
        Ok(())
    }
}

#[cfg(test)]
pub(crate) async fn checks(jvm: &Jvm) {
    use std::fs;
    let directory = Arc::new(tempfile::tempdir().unwrap());
    let slow = directory.path().join("slow");
    fs::write(&slow, "slow").unwrap();
    let weak = Arc::downgrade(&directory);
    let owned = directory.clone();
    let worker = jvm.clone();
    let name = slow.to_string_lossy().into_owned();
    let handle = tokio::spawn(async move { worker.print(name, owned).await });
    let deadline = Instant::now() + Duration::from_secs(3);
    while !slow.with_file_name("slow.started").exists() {
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(jvm.download_series("[]".into()).await.unwrap(), "[]");
    // More than the removed 1 worker + 8 queued jobs can run independently.
    let mut concurrent = Vec::new();
    for index in 0..16 {
        let path = directory.path().join(format!("concurrent-{index}"));
        fs::write(&path, "fast").unwrap();
        let owned = directory.clone();
        let worker = jvm.clone();
        concurrent.push(tokio::spawn(async move {
            worker
                .print(path.to_string_lossy().into_owned(), owned)
                .await
        }));
    }
    for call in concurrent {
        assert_eq!(call.await.unwrap().unwrap(), "{}");
    }
    drop(directory);
    assert_eq!(handle.await.unwrap(), Err(Error::Timeout));
    assert!(weak.upgrade().is_some(), "timed out call retains input");
    let other = Arc::new(tempfile::tempdir().unwrap());
    let fast = other.path().join("after-timeout");
    fs::write(&fast, "fast").unwrap();
    assert_eq!(
        jvm.print(fast.to_string_lossy().into_owned(), other.clone())
            .await
            .unwrap(),
        "{}"
    );
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert!(
        weak.upgrade().is_none(),
        "input released after native completion"
    );
    let cancelled = other.path().join("cancelled");
    fs::write(&cancelled, "slow").unwrap();
    let owned = other.clone();
    let worker = jvm.clone();
    let name = cancelled.to_string_lossy().into_owned();
    let waiter = tokio::spawn(async move { worker.print(name, owned).await });
    let deadline = Instant::now() + Duration::from_secs(3);
    while !cancelled.with_file_name("cancelled.started").exists() {
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    waiter.abort();
    let _ = waiter.await;
    let weak = Arc::downgrade(&other);
    drop(other);
    assert!(weak.upgrade().is_some());
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert!(weak.upgrade().is_none());
    let errors = Arc::new(tempfile::tempdir().unwrap());
    let expired = errors.path().join("expired");
    fs::write(&expired, "fast").unwrap();
    let (reply, receiver) = oneshot::channel();
    let runtime = jvm.0.print_runtime.borrow().clone().unwrap();
    runtime
        .start(Job {
            manifest: expired.to_string_lossy().into_owned(),
            _owner: errors.clone(),
            deadline: Instant::now() - Duration::from_millis(1),
            reply,
        })
        .unwrap();
    assert_eq!(receiver.await.unwrap(), Err(Error::Timeout));
    assert!(!expired.with_file_name("expired.started").exists());
    let path = errors.path().join("throw");
    fs::write(&path, "throw").unwrap();
    assert_eq!(
        jvm.print(path.to_string_lossy().into_owned(), errors).await,
        Err(Error::JavaException)
    );
    let directory = Arc::new(tempfile::tempdir().unwrap());
    let path = directory.path().join("shutdown");
    fs::write(&path, "slow").unwrap();
    let owner = directory.clone();
    let worker = jvm.clone();
    let name = path.to_string_lossy().into_owned();
    let waiter = tokio::spawn(async move { worker.print(name, owner).await });
    let deadline = Instant::now() + Duration::from_secs(3);
    while !path.with_file_name("shutdown.started").exists() {
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    jvm.shutdown().await.unwrap();
    assert!(
        path.with_file_name("shutdown.done").exists(),
        "shutdown waits for Java completion"
    );
    assert_eq!(waiter.await.unwrap(), Err(Error::Timeout));
}
