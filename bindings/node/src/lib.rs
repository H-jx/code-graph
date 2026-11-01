//! JSON transport and lifecycle only; one owner thread creates/uses/drops Core connections.
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use napi::bindgen_prelude::{AsyncBlock, AsyncBlockBuilder, Env};
use napi_derive::napi;
use project_knowledge_core::client;
use project_knowledge_core::contract::{CoreError, codes};
use project_knowledge_core::project::LoadedProject;
use project_knowledge_core::storage::Database;
use tokio_util::sync::CancellationToken;

fn native_error(error: CoreError) -> napi::Error {
    napi::Error::from_reason(serde_json::to_string(&error.to_envelope()).expect("error envelope"))
}

fn internal() -> CoreError {
    CoreError::new(codes::INTERNAL_ERROR, "native operation failed")
}

fn contain<T>(work: impl FnOnce() -> Result<T, CoreError>) -> Result<T, CoreError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).unwrap_or_else(|_| Err(internal()))
}

#[derive(Default)]
struct State {
    closed: bool,
    pending: BTreeMap<String, CancellationToken>,
}

struct Shared {
    state: Mutex<State>,
    done: tokio::sync::watch::Sender<bool>,
}

impl Shared {
    fn close(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.closed = true;
        for cancel in state.pending.values() {
            cancel.cancel();
        }
    }
}

struct Job {
    id: String,
    raw: String,
    maintenance: bool,
    cancel: CancellationToken,
    reply: tokio::sync::oneshot::Sender<Result<String, CoreError>>,
}

#[napi]
pub struct NativeClient {
    shared: Arc<Shared>,
    sender: mpsc::SyncSender<Job>,
    maintenance: bool,
}

impl Drop for NativeClient {
    fn drop(&mut self) {
        self.shared.close();
    }
}

#[napi]
impl NativeClient {
    #[napi(factory)]
    pub async fn open(config_path: String, maintenance: bool) -> napi::Result<Self> {
        let (done, _) = tokio::sync::watch::channel(false);
        let shared = Arc::new(Shared {
            state: Mutex::new(State::default()),
            done,
        });
        let (sender, receiver) = mpsc::sync_channel::<Job>(64);
        let (ready, opened) = tokio::sync::oneshot::channel();
        let owner = shared.clone();
        std::thread::Builder::new()
            .name("project-knowledge-owner".into())
            .spawn(move || {
                let startup = contain(|| {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|_| internal())?;
                    let absolute = std::path::absolute(&config_path).map_err(|_| {
                        CoreError::new(codes::CONFIG_INVALID, "cannot resolve config path")
                    })?;
                    let project = LoadedProject::load(&absolute)?;
                    let path = client::database_path(&project.config_path);
                    let database = if maintenance {
                        Database::open_maintenance(
                            &path,
                            &project.config.project_id,
                            &project.config_hash()?,
                            &client::now(),
                        )?
                    } else {
                        Database::open_readonly(&path)?
                    };
                    drop(database);
                    Ok((runtime, project.config_path))
                });
                match startup {
                    Err(error) => {
                        let _ = ready.send(Err(error));
                    }
                    Ok((runtime, path)) => {
                        let _ = ready.send(Ok(()));
                        loop {
                            let job = match receiver.recv_timeout(Duration::from_millis(25)) {
                                Ok(job) => job,
                                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                                Err(mpsc::RecvTimeoutError::Timeout) => {
                                    if owner.state.lock().unwrap_or_else(|e| e.into_inner()).closed
                                    {
                                        break;
                                    }
                                    continue;
                                }
                            };
                            let result = contain(|| {
                                client::check_cancel(&job.cancel)?;
                                let project = LoadedProject::load(&path)?;
                                let db_path = client::database_path(&path);
                                let mut database = if job.maintenance {
                                    Database::open_maintenance(
                                        &db_path,
                                        &project.config.project_id,
                                        &project.config_hash()?,
                                        &client::now(),
                                    )?
                                } else {
                                    Database::open_readonly(&db_path)?
                                };
                                client::check_cancel(&job.cancel)?;
                                let value = if job.maintenance {
                                    runtime.block_on(client::execute_maintenance(
                                        &project,
                                        &mut database,
                                        &job.raw,
                                        &job.cancel,
                                    ))?
                                } else {
                                    client::execute_query(
                                        &project,
                                        &mut database,
                                        &job.raw,
                                        &job.cancel,
                                        &job.id,
                                    )?
                                };
                                serde_json::to_string(&value).map_err(|_| internal())
                            });
                            owner
                                .state
                                .lock()
                                .unwrap_or_else(|e| e.into_inner())
                                .pending
                                .remove(&job.id);
                            let _ = job.reply.send(result);
                        }
                        drop(runtime);
                    }
                }
                owner.close();
                owner.done.send_replace(true);
            })
            .map_err(|_| native_error(internal()))?;
        opened
            .await
            .map_err(|_| native_error(internal()))?
            .map_err(native_error)?;
        Ok(Self {
            shared,
            sender,
            maintenance,
        })
    }

    /// Register synchronously before returning a Promise so cancel cannot race task startup.
    #[napi(ts_return_type = "Promise<string>")]
    pub fn execute(
        &self,
        env: Env,
        request_id: String,
        raw: String,
        maintenance: bool,
    ) -> napi::Result<AsyncBlock<String>> {
        if maintenance && !self.maintenance {
            return Err(native_error(CoreError::new(
                codes::INVALID_REQUEST,
                "maintenance requires an explicit maintenance client",
            )));
        }
        if raw.len() > 1_048_576 || request_id.is_empty() || request_id.len() > 128 {
            return Err(native_error(CoreError::new(
                codes::INVALID_REQUEST,
                "invalid request size or request ID",
            )));
        }
        let mut state = self.shared.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.closed {
            return Err(native_error(CoreError::new(
                codes::CLOSED,
                "client is closed",
            )));
        }
        if state.pending.contains_key(&request_id) {
            return Err(native_error(CoreError::new(
                codes::INVALID_REQUEST,
                "request ID is already in use",
            )));
        }
        let cancel = CancellationToken::new();
        let (reply, response) = tokio::sync::oneshot::channel();
        let job = Job {
            id: request_id.clone(),
            raw,
            maintenance,
            cancel: cancel.clone(),
            reply,
        };
        self.sender.try_send(job).map_err(|error| {
            native_error(CoreError::new(
                if matches!(error, mpsc::TrySendError::Full(_)) {
                    codes::BUSY
                } else {
                    codes::CLOSED
                },
                "native request queue unavailable",
            ))
        })?;
        state.pending.insert(request_id, cancel);
        drop(state);
        AsyncBlockBuilder::new(async move {
            response
                .await
                .map_err(|_| native_error(internal()))?
                .map_err(native_error)
        })
        .build(&env)
    }

    #[napi]
    pub fn cancel(&self, request_id: String) -> bool {
        let state = self.shared.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cancel) = state.pending.get(&request_id) {
            cancel.cancel();
            true
        } else {
            false
        }
    }

    #[napi(ts_return_type = "Promise<void>")]
    pub fn close(&self, env: Env) -> napi::Result<AsyncBlock<()>> {
        self.shared.close();
        let mut done = self.shared.done.subscribe();
        AsyncBlockBuilder::new(async move {
            tokio::time::timeout(Duration::from_secs(10), done.wait_for(|finished| *finished))
                .await
                .map_err(|_| {
                    native_error(CoreError::retryable(
                        codes::BUSY,
                        "close exceeded 10000 ms; client remains closed while cleanup finishes",
                    ))
                })?
                .map_err(|_| native_error(internal()))?;
            Ok(())
        })
        .build(&env)
    }
}

#[napi]
pub async fn get_capabilities() -> napi::Result<String> {
    tokio::task::spawn_blocking(|| contain(|| Ok(client::capabilities().to_string())))
        .await
        .map_err(|_| native_error(internal()))?
        .map_err(native_error)
}

/// Validate transport arguments with Core without opening a project or database.
#[napi]
pub async fn validate_request(raw: String, maintenance: bool) -> napi::Result<()> {
    tokio::task::spawn_blocking(move || {
        contain(|| {
            if maintenance {
                client::decode::<project_knowledge_core::contract::client::MaintenanceRequest>(
                    &raw,
                )?;
            } else {
                client::decode::<project_knowledge_core::contract::QueryRequest>(&raw)?;
            }
            Ok(())
        })
    })
    .await
    .map_err(|_| native_error(internal()))?
    .map_err(native_error)
}

#[cfg(test)]
mod tests {
    #[test]
    fn panic_is_contained_without_exposing_payload() {
        let error = super::contain::<()>(|| panic!("private payload")).unwrap_err();
        assert_eq!(error.code, "INTERNAL_ERROR");
        assert!(!error.message.contains("private"));
    }
}
