use crate::{
    auth::UserDetail,
    server::{
        chancomms::ControlChanMsg,
        controlchan::{
            error::ControlChanError,
            handler::{CommandContext, CommandHandler},
        },
        controlchan::{Reply, ReplyCode},
    },
    storage::{Metadata, StorageBackend},
};
use async_trait::async_trait;
use futures::{channel::mpsc::Sender, prelude::*};
use std::{path::PathBuf, sync::Arc};

#[derive(Debug)]
pub struct Size {
    path: PathBuf,
}

impl Size {
    pub fn new(path: PathBuf) -> Self {
        Size { path }
    }
}

#[async_trait]
impl<Storage, User> CommandHandler<Storage, User> for Size
where
    User: UserDetail,
    Storage: StorageBackend<User> + 'static,
    Storage::Metadata: 'static + Metadata,
{
    #[tracing_attributes::instrument]
    async fn handle(&self, args: CommandContext<Storage, User>) -> Result<Reply, ControlChanError> {
        let session = args.session.lock().await;
        let user = session.user.clone();
        let start_pos: u64 = session.start_pos;
        let storage: Arc<Storage> = Arc::clone(&session.storage);
        let path = session.cwd.join(self.path.clone());
        let mut tx_success: Sender<ControlChanMsg> = args.tx_control_chan.clone();
        let mut tx_fail: Sender<ControlChanMsg> = args.tx_control_chan.clone();
        let logger = args.logger;

        tokio::spawn(async move {
            match storage.metadata(&user, &path).await {
                Ok(metadata) => {
                    if let Err(err) = tx_success
                        .send(ControlChanMsg::CommandChannelReply(Reply::new_with_string(
                            ReplyCode::FileStatus,
                            (metadata.len() - start_pos).to_string(),
                        )))
                        .await
                    {
                        slog::warn!(logger, "{}", err);
                    }
                }
                Err(err) => {
                    if let Err(err) = tx_fail.send(ControlChanMsg::StorageError(err)).await {
                        slog::warn!(logger, "{}", err);
                    }
                }
            }
        });
        Ok(Reply::none())
    }
}
