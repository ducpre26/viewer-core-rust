//! Owned jobs stay on the worker through completion, even after waiter cancellation.
use super::*;
use std::{path::PathBuf, time::Instant};

pub enum FileOperation {
    Video { fps: i32 },
}

/// Paths are server-generated. The owner retains the job directory through
/// completion; dropping an HTTP future cannot delete active Java input/output.
pub struct FileRequest {
    pub operation: FileOperation,
    pub input: PathBuf,
    pub output: PathBuf,
    pub owner: Arc<dyn Send + Sync>,
}

enum Work {
    KeyImage {
        operation: String,
        manifest: String,
        _owner: Arc<dyn Send + Sync>,
    },
    Metadata {
        json: String,
        pattern: String,
        index: String,
    },
    Series {
        json: String,
    },
    File(FileRequest),
}

pub(super) struct Job {
    work: Work,
    deadline: Instant,
    reply: oneshot::Sender<Result<String, Error>>,
    _active: Active,
}
struct Active(Arc<std::sync::atomic::AtomicUsize>);
impl Drop for Active {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Jvm {
    async fn download(&self, work: Work, deadline: Instant) -> Result<String, Error> {
        if *self.0.state.borrow() != State::Ready {
            return Err(Error::WorkerStopped);
        }
        if deadline <= Instant::now() {
            return Err(Error::Timeout);
        }
        let (reply, receiver) = oneshot::channel();
        self.0.active.fetch_add(1, Ordering::SeqCst);
        let job = Job {
            work,
            deadline,
            reply,
            _active: Active(self.0.active.clone()),
        };
        self.0
            .sender
            .try_send(Command::Download(job))
            .map_err(|e| match e {
                mpsc::error::TrySendError::Full(_) => Error::Busy,
                mpsc::error::TrySendError::Closed(_) => Error::WorkerStopped,
            })?;
        // Timing out drops only the receiver. The worker retains the job/lease.
        tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), receiver)
            .await
            .map_err(|_| Error::Timeout)?
            .map_err(|_| Error::WorkerStopped)?
    }
    pub async fn key_image(
        &self,
        operation: String,
        manifest: String,
        owner: Arc<dyn Send + Sync>,
    ) -> Result<String, Error> {
        if !self
            .0
            .capabilities
            .borrow()
            .as_ref()
            .is_some_and(|c| c.operations.iter().any(|o| o == "keyImage"))
        {
            return Err(Error::Contract);
        }
        self.download(
            Work::KeyImage {
                operation,
                manifest,
                _owner: owner,
            },
            Instant::now() + self.0.timeout,
        )
        .await
    }
    pub async fn download_metadata(
        &self,
        json: String,
        pattern: String,
        index: String,
    ) -> Result<String, Error> {
        self.download(
            Work::Metadata {
                json,
                pattern,
                index,
            },
            Instant::now() + self.0.timeout,
        )
        .await
    }
    pub async fn download_series(&self, json: String) -> Result<String, Error> {
        self.download(Work::Series { json }, Instant::now() + self.0.timeout)
            .await
    }
    pub async fn download_file(&self, request: FileRequest) -> Result<(), Error> {
        self.download(Work::File(request), Instant::now() + self.0.timeout)
            .await
            .map(|_| ())
    }
}

pub(super) fn execute(
    env: &mut JNIEnv<'_>,
    class: &GlobalRef,
    job: Job,
    running: &watch::Sender<Option<Instant>>,
) -> Result<(), Error> {
    if job.reply.is_closed() || Instant::now() >= job.deadline {
        let _ = job.reply.send(Err(Error::Timeout));
        return Ok(());
    }
    running.send_replace(Some(job.deadline));
    let result = frame(env, |env| {
        let class: &JClass<'_> = class.as_obj().into();
        match &job.work {
            Work::KeyImage {
                operation,
                manifest,
                ..
            } => {
                let operation = invoke(env, |e| e.new_string(operation))?;
                let manifest = invoke(env, |e| e.new_string(manifest))?;
                let value = invoke(env, |e| {
                    e.call_static_method(
                        class,
                        "keyImage",
                        "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
                        &[(&operation).into(), (&manifest).into()],
                    )
                })?
                .l()?;
                text(env, value)
            }
            Work::Metadata {
                json,
                pattern,
                index,
            } => {
                let json = invoke(env, |e| e.new_string(json))?;
                let pattern = invoke(env, |e| e.new_string(pattern))?;
                let index = invoke(env, |e| e.new_string(index))?;
                let value = invoke(env, |e| {
                    e.call_static_method(class,"downloadMetadata",
                    "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
                    &[(&json).into(),(&pattern).into(),(&index).into()])
                })?
                .l()?;
                text(env, value)
            }
            Work::Series { json } => {
                let json = invoke(env, |e| e.new_string(json))?;
                let value = invoke(env, |e| {
                    e.call_static_method(
                        class,
                        "downloadSeries",
                        "(Ljava/lang/String;)Ljava/lang/String;",
                        &[(&json).into()],
                    )
                })?
                .l()?;
                text(env, value)
            }
            Work::File(request) => {
                let input = std::fs::canonicalize(&request.input).map_err(|_| Error::Contract)?;
                let parent = std::fs::canonicalize(request.output.parent().ok_or(Error::Contract)?)
                    .map_err(|_| Error::Contract)?;
                if input.parent() != Some(parent.as_path()) || request.output.exists() {
                    return Err(Error::Contract);
                }
                let source = invoke(env, |e| e.new_string(request.input.to_str().unwrap_or("")))?;
                let output = invoke(env, |e| e.new_string(request.output.to_str().unwrap_or("")))?;
                let mut args = vec![(&source).into(), (&output).into()];
                let (method, signature) = match request.operation {
                    FileOperation::Video { fps } => {
                        args.push(fps.into());
                        ("encodeVideo", "(Ljava/lang/String;Ljava/lang/String;I)V")
                    }
                };
                invoke(env, |e| {
                    e.call_static_method(class, method, signature, &args)
                })?;
                Ok(String::new())
            }
        }
    });
    let fatal = matches!(result, Err(Error::Unrecoverable));
    running.send_replace(None);
    let _ = job.reply.send(result);
    if fatal {
        Err(Error::Unrecoverable)
    } else {
        Ok(())
    }
}
fn text(env: &mut JNIEnv<'_>, value: jni::objects::JObject<'_>) -> Result<String, Error> {
    let value = JString::from(value);
    Ok(invoke(env, |e| e.get_string(&value))?.into())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::fs;

    pub(crate) async fn key_image_checks(jvm: &Jvm) {
        // All paths are test-owned; the fixture proves JNI entered before timeout/drop.
        let directory = Arc::new(tempfile::tempdir().unwrap());
        let path = directory
            .path()
            .join("input")
            .to_string_lossy()
            .into_owned();
        let weak = Arc::downgrade(&directory);
        let result = jvm
            .download(
                Work::KeyImage {
                    operation: "slow".into(),
                    manifest: path.clone(),
                    _owner: directory.clone(),
                },
                Instant::now() + Duration::from_millis(200),
            )
            .await;
        assert_eq!(result, Err(Error::Timeout));
        assert!(std::path::Path::new(&format!("{path}.started")).exists());
        drop(directory);
        assert!(weak.upgrade().is_some());
        tokio::time::sleep(Duration::from_millis(800)).await;
        assert!(weak.upgrade().is_none());
        let directory = Arc::new(tempfile::tempdir().unwrap());
        let path = directory
            .path()
            .join("input")
            .to_string_lossy()
            .into_owned();
        let owner = directory.clone();
        let worker = jvm.clone();
        let name = path.clone();
        let task = tokio::spawn(async move { worker.key_image("slow".into(), name, owner).await });
        tokio::time::timeout(Duration::from_secs(2), async {
            while !std::path::Path::new(&format!("{path}.started")).exists() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        // Saturate the same bounded queue with expired jobs; none may enter Java.
        let mut tasks = Vec::new();
        for n in 0..20 {
            let worker = jvm.clone();
            let owner = directory.clone();
            let name = format!("{path}-{n}");
            tasks.push(tokio::spawn(async move {
                worker
                    .download(
                        Work::KeyImage {
                            operation: "queued".into(),
                            manifest: name,
                            _owner: owner,
                        },
                        Instant::now() + Duration::from_millis(100),
                    )
                    .await
            }));
        }
        let mut busy = 0;
        let mut expired = 0;
        for task in tasks {
            match task.await.unwrap() {
                Err(Error::Busy) => busy += 1,
                Err(Error::Timeout) => expired += 1,
                other => panic!("unexpected {other:?}"),
            }
        }
        assert!(busy > 0 && expired > 0);
        task.abort();
        let _ = task.await;
        let weak = Arc::downgrade(&directory);
        drop(directory);
        assert!(weak.upgrade().is_some());
        tokio::time::sleep(Duration::from_millis(800)).await;
        assert!(weak.upgrade().is_none());
        let owner = Arc::new(tempfile::tempdir().unwrap());
        let path = owner.path().join("input").to_string_lossy().into_owned();
        assert_eq!(
            jvm.key_image("count".into(), path.clone(), owner.clone())
                .await
                .unwrap(),
            "{\"count\":2}",
            "expired/cancelled queue work must not invoke Java"
        );
        for _ in 0..100 {
            assert_eq!(
                jvm.key_image("throw".into(), path.clone(), owner.clone())
                    .await,
                Err(Error::JavaException)
            );
        }
        assert!(jvm.check().await.is_ok());
    }
    pub(crate) async fn download_checks(jvm: &Jvm) {
        use tokio::sync::Semaphore;
        struct Owner {
            directory: tempfile::TempDir,
            _permit: tokio::sync::OwnedSemaphorePermit,
        }
        let slots = Arc::new(Semaphore::new(1));
        let owner = Arc::new(Owner {
            directory: tempfile::tempdir().unwrap(),
            _permit: slots.clone().try_acquire_owned().unwrap(),
        });
        let root = owner.directory.path().to_owned();
        let input = root.join("input");
        fs::write(&input, "slow").unwrap();
        let weak = Arc::downgrade(&owner);
        let request = FileRequest {
            operation: FileOperation::Video { fps: 1 },
            input: input.clone(),
            output: root.join("output"),
            owner: owner.clone(),
        };
        let result = jvm
            .download(
                Work::File(request),
                Instant::now() + Duration::from_millis(200),
            )
            .await;
        assert_eq!(result, Err(Error::Timeout));
        assert!(
            root.join("input.started").exists(),
            "Java actually entered the slow operation"
        );
        drop(owner);
        assert!(weak.upgrade().is_some());
        assert!(root.exists());
        assert_eq!(slots.available_permits(), 0);
        assert_eq!(
            jvm.check().await,
            Err(Error::Timeout),
            "Overdue native call makes readiness unavailable without releasing its lease"
        );
        tokio::time::sleep(Duration::from_millis(800)).await;
        assert!(weak.upgrade().is_none());
        assert!(!root.exists());
        assert_eq!(slots.available_permits(), 1);

        let directory = Arc::new(tempfile::tempdir().unwrap());
        let input = directory.path().join("input");
        fs::write(&input, "slow").unwrap();
        let worker = jvm.clone();
        let lease = directory.clone();
        let source = input.clone();
        let running = tokio::spawn(async move {
            worker
                .download(
                    Work::File(FileRequest {
                        operation: FileOperation::Video { fps: 1 },
                        input: source,
                        output: lease.path().join("output"),
                        owner: lease,
                    }),
                    Instant::now() + Duration::from_secs(5),
                )
                .await
        });
        let started = input.with_file_name("input.started");
        tokio::time::timeout(Duration::from_secs(3), async {
            while !started.exists() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        let mut queued = Vec::new();
        for _ in 0..20 {
            let worker = jvm.clone();
            queued.push(tokio::spawn(async move {
                worker
                    .download(
                        Work::Series { json: "[]".into() },
                        Instant::now() + Duration::from_millis(100),
                    )
                    .await
            }));
        }
        let mut busy = 0;
        let mut timeout = 0;
        for task in queued {
            match task.await.unwrap() {
                Err(Error::Busy) => busy += 1,
                Err(Error::Timeout) => timeout += 1,
                other => panic!("unexpected queue result {other:?}"),
            }
        }
        assert!(
            busy > 0 && timeout > 0,
            "bounded admission and queued deadlines"
        );
        running.await.unwrap().unwrap();
        fs::write(&input, "throw").unwrap();
        let result = jvm
            .download(
                Work::File(FileRequest {
                    operation: FileOperation::Video { fps: 1 },
                    input: input.clone(),
                    output: directory.path().join("error-output"),
                    owner: directory.clone(),
                }),
                Instant::now() + Duration::from_secs(3),
            )
            .await;
        assert_eq!(result, Err(Error::JavaException));
        for _ in 0..2000 {
            assert_eq!(
                jvm.download(
                    Work::Series { json: "[]".into() },
                    Instant::now() + Duration::from_secs(3)
                )
                .await
                .unwrap(),
                "[]"
            );
        }
        assert!(jvm.check().await.is_ok());
    }
}
