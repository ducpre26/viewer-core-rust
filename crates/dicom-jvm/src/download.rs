//! Owned jobs stay on the worker through completion, even after waiter cancellation.
use super::*;
use std::{path::PathBuf, time::Instant};

#[derive(Clone, Copy)]
pub struct FileLimits {
    pub input_bytes: i64,
    pub output_bytes: i64,
    pub pixels: i64,
    pub frames: i32,
}

pub enum FileOperation {
    Video { fps: i32 },
}

/// Paths are server-generated. The owner retains the job directory AND admission
/// permit; dropping an HTTP future cannot delete an active Java input/output.
pub struct FileRequest {
    pub operation: FileOperation,
    pub input: PathBuf,
    pub output: PathBuf,
    pub limits: FileLimits,
    pub owner: Arc<dyn Send + Sync>,
}

enum Work {
    Metadata {
        json: String,
        pattern: String,
        index: String,
        max: i32,
    },
    Series {
        json: String,
        max: i32,
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
    pub async fn download_metadata(
        &self,
        json: String,
        pattern: String,
        index: String,
        max: i32,
        deadline: Instant,
    ) -> Result<String, Error> {
        if json.len() > 16 * 1024 * 1024 || pattern.len() > 4096 || index.len() > 128 || max <= 0 {
            return Err(Error::Contract);
        }
        self.download(
            Work::Metadata {
                json,
                pattern,
                index,
                max,
            },
            deadline,
        )
        .await
    }
    pub async fn download_series(
        &self,
        json: String,
        max: i32,
        deadline: Instant,
    ) -> Result<String, Error> {
        if json.len() > 16 * 1024 * 1024 || max <= 0 {
            return Err(Error::Contract);
        }
        self.download(Work::Series { json, max }, deadline).await
    }
    pub async fn download_file(
        &self,
        request: FileRequest,
        deadline: Instant,
    ) -> Result<(), Error> {
        self.download(Work::File(request), deadline)
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
            Work::Metadata {
                json,
                pattern,
                index,
                max,
            } => {
                let json = invoke(env, |e| e.new_string(json))?;
                let pattern = invoke(env, |e| e.new_string(pattern))?;
                let index = invoke(env, |e| e.new_string(index))?;
                let value = invoke(env, |e| {
                    e.call_static_method(class,"downloadMetadata",
                    "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;I)Ljava/lang/String;",
                    &[(&json).into(),(&pattern).into(),(&index).into(),(*max).into()])
                })?
                .l()?;
                text(env, value)
            }
            Work::Series { json, max } => {
                let json = invoke(env, |e| e.new_string(json))?;
                let value = invoke(env, |e| {
                    e.call_static_method(
                        class,
                        "downloadSeries",
                        "(Ljava/lang/String;I)Ljava/lang/String;",
                        &[(&json).into(), (*max).into()],
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
                let limits = request.limits;
                let mut args = vec![(&source).into(), (&output).into()];
                let (method, signature) = match request.operation {
                    FileOperation::Video { fps } => {
                        args.push(fps.into());
                        (
                            "encodeVideo",
                            "(Ljava/lang/String;Ljava/lang/String;IJJJI)V",
                        )
                    }
                };
                args.push(limits.input_bytes.into());
                args.push(limits.output_bytes.into());
                args.push(limits.pixels.into());
                args.push(limits.frames.into());
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
    let length = invoke(env, |e| e.call_method(&value, "length", "()I", &[]))?.i()?;
    if !(0..=16 * 1024 * 1024).contains(&length) {
        return Err(Error::Contract);
    }
    Ok(invoke(env, |e| e.get_string(&value))?.into())
}
