//! One embedded JVM per process with dedicated control/download and print workers.
//! No Java references or environments cross this crate's public boundary.
mod config;
mod download;
mod print;
pub use config::{JvmConfig, JvmSettings};
pub use download::{FileOperation, FileRequest};

use jni::{
    InitArgsBuilder, JNIEnv, JNIVersion, JavaVM,
    objects::{GlobalRef, JClass, JString},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::{mpsc, oneshot, watch};

static JVM_ATTEMPTED: AtomicBool = AtomicBool::new(false);
const FACADE: &str = "vn/saolasoft/pacs/dicom/DicomFacade";
// Shared bounded queue for control and owned download work; one native worker.
const CONTROL_QUEUE: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    RuntimeConfig,
    JavaVersion,
    Artifact,
    Contract,
    AlreadyStarted,
    Startup,
    Jni,
    JavaException,
    ExceptionDetailsUnavailable,
    Unrecoverable,
    Busy,
    Timeout,
    WorkerStopped,
    Shutdown,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::RuntimeConfig => "JVM_CONFIG: invalid or missing explicit runtime configuration",
            Self::JavaVersion => "JVM_VERSION: Java 25 required",
            Self::Artifact => "JVM_ARTIFACT: invalid distribution, classpath or checksum",
            Self::Contract => "JVM_CONTRACT: facade API/capabilities mismatch",
            Self::AlreadyStarted => {
                "JVM_ONCE: a JVM startup was already attempted; restart process"
            }
            Self::Startup => "JVM_STARTUP: could not load or create configured JVM",
            Self::Jni => "JNI_CALL: JNI operation failed",
            Self::JavaException => "JAVA_EXCEPTION: Java call threw; details withheld",
            Self::ExceptionDetailsUnavailable => {
                "JAVA_EXCEPTION_FALLBACK: exception inspection failed; details withheld"
            }
            Self::Unrecoverable => "JVM_FATAL: virtual machine failure; restart process",
            Self::Busy => "JVM_BUSY: worker queue full",
            Self::Timeout => "JVM_TIMEOUT: worker deadline exceeded",
            Self::WorkerStopped => "JVM_STOPPED: worker unavailable",
            Self::Shutdown => "JVM_SHUTDOWN: did not stop within limit; terminate process",
        })
    }
}
impl std::error::Error for Error {}
impl From<jni::errors::Error> for Error {
    fn from(_: jni::errors::Error) -> Self {
        Self::Jni
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Capabilities {
    pub api_version: u32,
    pub operations: Vec<String>,
    pub codecs: Vec<String>,
}
impl Capabilities {
    fn is_supported(&self) -> bool {
        self.api_version == 2
            && self.codecs.is_empty()
            && (self.operations.is_empty()
                || self.operations == ["downloadMetadata", "downloadSeries", "encodeVideo"]
                || self.operations
                    == [
                        "downloadMetadata",
                        "downloadSeries",
                        "encodeVideo",
                        "keyImage",
                    ]
                || self.operations
                    == [
                        "downloadMetadata",
                        "downloadSeries",
                        "encodeVideo",
                        "keyImage",
                        "print",
                    ])
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum State {
    Starting,
    Ready,
    Stopping,
    Stopped,
    Failed,
}

enum Command {
    Download(download::Job),
    Probe(oneshot::Sender<Result<Capabilities, Error>>),
    Stop(oneshot::Sender<Result<(), Error>>),
}

struct Shared {
    print_runtime: watch::Receiver<Option<Arc<print::Runtime>>>,
    active: Arc<std::sync::atomic::AtomicUsize>,
    capabilities: watch::Sender<Option<Capabilities>>,
    running: watch::Sender<Option<std::time::Instant>>,
    sender: mpsc::Sender<Command>,
    state: watch::Sender<State>,
    timeout: Duration,
    shutdown_timeout: Duration,
}

/// Cloneable handle. Native workers own VM attachments and Java references.
#[derive(Clone)]
pub struct Jvm(Arc<Shared>);

impl Jvm {
    pub fn supports_download(&self) -> bool {
        self.0.capabilities.borrow().as_ref().is_some_and(|c| {
            c.operations
                .iter()
                .any(|operation| operation == "encodeVideo")
        })
    }
    pub async fn start(config: JvmConfig) -> Result<Self, Error> {
        if JVM_ATTEMPTED.swap(true, Ordering::SeqCst) {
            return Err(Error::AlreadyStarted);
        }
        let (sender, receiver) = mpsc::channel(CONTROL_QUEUE);
        let (state, _) = watch::channel(State::Starting);
        let (ready_tx, ready_rx) = oneshot::channel();
        let (capabilities, _) = watch::channel(None);
        let (running, _) = watch::channel(None);
        let (print_ready, print_runtime) = watch::channel(None);
        let shared = Arc::new(Shared {
            print_runtime,
            active: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            capabilities: capabilities.clone(),
            running: running.clone(),
            sender,
            state: state.clone(),
            timeout: config.timeout,
            shutdown_timeout: config.shutdown_timeout,
        });
        std::thread::Builder::new()
            .name("dicom-jni".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    worker(
                        config,
                        receiver,
                        &state,
                        ready_tx,
                        capabilities,
                        running,
                        print_ready,
                    )
                }));
                if !matches!(result, Ok(Ok(()))) {
                    state.send_replace(State::Failed);
                }
            })
            .map_err(|_| Error::Startup)?;
        match tokio::time::timeout(shared.timeout, ready_rx).await {
            Ok(Ok(Ok(()))) => Ok(Self(shared)),
            Ok(Ok(Err(error))) => Err(error),
            Ok(Err(_)) => Err(Error::WorkerStopped),
            Err(_) => {
                shared.state.send_replace(State::Failed);
                Err(Error::Timeout)
            }
        }
    }

    /// Recheck version/capabilities on the JNI worker, without blocking Tokio.
    pub async fn check(&self) -> Result<Capabilities, Error> {
        if *self.0.state.borrow() != State::Ready {
            return Err(Error::WorkerStopped);
        }
        let (reply, receive) = oneshot::channel();
        // A healthy codec call may outlast a control probe timeout. Use the last
        // verified capabilities while business work owns the sole JNI worker.
        if self.0.active.load(Ordering::SeqCst) > 0 {
            if self
                .0
                .running
                .borrow()
                .is_some_and(|deadline| std::time::Instant::now() >= deadline)
            {
                return Err(Error::Timeout);
            }
            return self
                .0
                .capabilities
                .borrow()
                .clone()
                .ok_or(Error::WorkerStopped);
        }
        self.0
            .sender
            .try_send(Command::Probe(reply))
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => Error::Busy,
                mpsc::error::TrySendError::Closed(_) => Error::WorkerStopped,
            })?;
        match tokio::time::timeout(self.0.timeout, receive).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => {
                self.0.state.send_replace(State::Failed);
                Err(Error::WorkerStopped)
            }
            Err(_) => {
                if self.0.active.load(Ordering::SeqCst) > 0 {
                    return Err(Error::Timeout);
                }
                self.0.state.send_replace(State::Failed);
                Err(Error::Timeout)
            }
        }
    }

    /// Completes only on an unrecoverable worker failure; the host must exit.
    pub async fn wait_failed(&self) {
        let mut receiver = self.0.state.subscribe();
        loop {
            if *receiver.borrow_and_update() == State::Failed {
                return;
            }
            if receiver.changed().await.is_err() {
                return;
            }
        }
    }

    /// No blocking thread join on Tokio. The host must exit if this returns an error.
    pub async fn shutdown(&self) -> Result<(), Error> {
        self.0.state.send_replace(State::Stopping);
        tokio::time::timeout(self.0.shutdown_timeout, async {
            let (reply, receive) = oneshot::channel();
            self.0
                .sender
                .send(Command::Stop(reply))
                .await
                .map_err(|_| Error::WorkerStopped)?;
            receive.await.map_err(|_| Error::WorkerStopped)?
        })
        .await
        .map_err(|_| Error::Shutdown)?
    }
}

fn create_vm(config: &JvmConfig) -> Result<JavaVM, Error> {
    let mut builder = InitArgsBuilder::new().version(JNIVersion::V8);
    for option in &config.options {
        builder = builder.option(option);
    }
    let args = builder.build().map_err(|_| Error::RuntimeConfig)?;
    JavaVM::with_libjvm(args, || Ok(&config.libjvm)).map_err(|_| Error::Startup)
}

fn worker(
    config: JvmConfig,
    mut receiver: mpsc::Receiver<Command>,
    state: &watch::Sender<State>,
    ready: oneshot::Sender<Result<(), Error>>,
    capabilities: watch::Sender<Option<Capabilities>>,
    running: watch::Sender<Option<std::time::Instant>>,
    print_ready: watch::Sender<Option<Arc<print::Runtime>>>,
) -> Result<(), Error> {
    let vm = match create_vm(&config) {
        Ok(vm) => vm,
        Err(error) => {
            let _ = ready.send(Err(error));
            return Err(error);
        }
    };
    let vm = Arc::new(vm);
    let print_runtime = print::Runtime::new(vm.clone(), state.clone());
    print_ready.send_replace(Some(print_runtime.clone()));
    let mut stop_reply = None;
    let work = (|| -> Result<(), Error> {
        let mut env = vm.attach_current_thread().map_err(|_| Error::Jni)?;
        let setup = frame(&mut env, |env| {
            let version_key = invoke(env, |env| env.new_string("java.specification.version"))?;
            let value = invoke(env, |env| {
                env.call_static_method(
                    "java/lang/System",
                    "getProperty",
                    "(Ljava/lang/String;)Ljava/lang/String;",
                    &[(&version_key).into()],
                )
            })?
            .l()?;
            let version: String = invoke(env, |env| env.get_string((&value).into()))?.into();
            if version != "25" {
                return Err(Error::JavaVersion);
            }
            let class = invoke(env, |env| env.find_class(FACADE))?;
            invoke(env, |env| env.new_global_ref(class))
        })
        .and_then(|class| {
            capabilities.send_replace(Some(probe(&mut env, &class)?));
            Ok(class)
        });
        let class = match setup {
            Ok(class) => class,
            Err(error) => {
                let _ = ready.send(Err(error));
                return Err(error);
            }
        };
        state.send_replace(State::Ready);
        if ready.send(Ok(())).is_err() {
            return Err(Error::WorkerStopped);
        }
        eprintln!("JVM ready: Java 25; facade API 2; JNI worker attached");
        while let Some(command) = receiver.blocking_recv() {
            match command {
                Command::Download(job) => download::execute(&mut env, &class, job, &running)?,
                Command::Probe(reply) => {
                    if reply.is_closed() {
                        continue;
                    }
                    let result = probe(&mut env, &class);
                    let failed = result.is_err();
                    let _ = reply.send(result);
                    if failed {
                        state.send_replace(State::Failed);
                        return Err(Error::Unrecoverable);
                    }
                }
                Command::Stop(reply) => {
                    stop_reply = Some(reply);
                    break;
                }
            }
        }
        // Declaration/drop order releases the GlobalRef while the worker is attached,
        // then the scoped guard detaches this thread. No JNIEnv leaves this scope.
        drop(class);
        drop(env);
        Ok(())
    })();
    print_ready.send_replace(None);
    print_runtime.shutdown()?;
    if work.is_err() {
        state.send_replace(State::Failed);
    }
    if vm.threads_attached() != 0 {
        return Err(Error::Shutdown);
    }
    eprintln!("JNI worker detached; global references released");
    // SAFETY: all print calls have completed. All JNI references
    // and scoped attachments have been dropped; there are no Rust daemon attaches.
    // Java non-daemon threads may delay destruction, bounded by the host's shutdown
    // deadline. No JNI operation is performed after destruction.
    let vm = Arc::try_unwrap(vm).map_err(|_| Error::Shutdown)?;
    let destroyed = unsafe { vm.destroy() }.map_err(|_| Error::Shutdown);
    if destroyed.is_ok() {
        eprintln!("JVM destroyed");
    }
    if let Some(reply) = stop_reply {
        state.send_replace(if destroyed.is_ok() {
            State::Stopped
        } else {
            State::Failed
        });
        let _ = reply.send(destroyed);
    }
    work.and(destroyed)
}

fn probe(env: &mut JNIEnv<'_>, class: &GlobalRef) -> Result<Capabilities, Error> {
    frame(env, |env| {
        let class: &JClass<'_> = class.as_obj().into();
        let version = invoke(env, |env| {
            env.call_static_method(class, "apiVersion", "()I", &[])
        })?
        .i()?;
        if version != 2 {
            return Err(Error::Contract);
        }
        let value = invoke(env, |env| {
            env.call_static_method(class, "capabilitiesJson", "()Ljava/lang/String;", &[])
        })?
        .l()?;
        let text = JString::from(value);
        // Foundation metadata only; reject unexpectedly large output before copying it.
        let length = invoke(env, |env| env.call_method(&text, "length", "()I", &[]))?.i()?;
        if !(0..=4096).contains(&length) {
            return Err(Error::Contract);
        }
        let json: String = invoke(env, |env| env.get_string(&text))?.into();
        let capabilities: Capabilities =
            serde_json::from_str(&json).map_err(|_| Error::Contract)?;
        if !capabilities.is_supported() {
            return Err(Error::Contract);
        }
        Ok(capabilities)
    })
}

fn frame<T>(
    env: &mut JNIEnv<'_>,
    operation: impl FnOnce(&mut JNIEnv<'_>) -> Result<T, Error>,
) -> Result<T, Error> {
    let result = env.with_local_frame(32, operation);
    if env.exception_check().map_err(|_| Error::Jni)? {
        // Also covers PushLocalFrame failure. Do not inspect after a failed allocation.
        env.exception_clear().map_err(|_| Error::Jni)?;
        return Err(Error::Unrecoverable);
    }
    result
}

fn invoke<'local, T>(
    env: &mut JNIEnv<'local>,
    operation: impl FnOnce(&mut JNIEnv<'local>) -> jni::errors::Result<T>,
) -> Result<T, Error> {
    let result = operation(env);
    if env.exception_check().map_err(|_| Error::Jni)? {
        let throwable = env.exception_occurred().map_err(|_| Error::Jni)?;
        env.exception_clear().map_err(|_| Error::Jni)?;
        // Examine the type and message only AFTER clearing. Message text never crosses
        // the boundary or reaches logs (it may contain PHI, credentials or paths).
        let details = (|| -> jni::errors::Result<bool> {
            let fatal = env.is_instance_of(&throwable, "java/lang/VirtualMachineError")?;
            env.call_method(&throwable, "getMessage", "()Ljava/lang/String;", &[])?;
            Ok(fatal)
        })();
        let secondary = env.exception_check().map_err(|_| Error::Jni)?;
        if secondary {
            env.exception_clear().map_err(|_| Error::Jni)?;
        }
        return Err(match (details, secondary) {
            (Ok(true), false) => Error::Unrecoverable,
            (Ok(false), false) => Error::JavaException,
            _ => Error::ExceptionDetailsUnavailable,
        });
    }
    result.map_err(|_| Error::Jni)
}

#[cfg(test)]
mod tests;
